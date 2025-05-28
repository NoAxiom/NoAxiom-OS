import argparse
import re

def parse_log_line(line_content):
    """
    解析单行日志以提取关键信息。
    如果匹配成功，会额外返回 metadata_tag_original_string 字段，
    该字段包含了原始日志中准确的 "[LEVEL, HART, TID at TIMEms]" 标记字符串。
    """
    # 正则表达式解释:
    # Group 1 (metadata_tag_original_string): 捕获整个 "[LEVEL, HART, TID at TIMEms]" 标记，包括括号和内部空格。
    # Group 2 (level): 捕获 INFO, DEBUG, 或 WARN。
    # Group 3 (hart_id): 捕获 HART ID 数字。
    # Group 4 (tid): 捕获 TID 数字。
    # Group 5 (timestamp_ms): 捕获时间戳数字。
    # Group 6 (message): 捕获标记之后的所有内容。
    match_general = re.search(
        r"(\[\s*(INFO|DEBUG|WARN)\s*,\s*HART(\d+)\s*,\s*TID(\d+)\s*at\s*(\d+)ms\])\s*(.*)", 
        line_content
    )
    if match_general:
        metadata_tag_original_string = match_general.group(1)
        level = match_general.group(2).strip()
        hart_id = match_general.group(3)
        tid = match_general.group(4)
        timestamp_ms = match_general.group(5)
        message = match_general.group(6).strip() 
        
        return {
            "type": "general",
            "hart": int(hart_id),
            "tid": int(tid),
            "time": int(timestamp_ms),
            "level": level, 
            "message": message, 
            "original_line": line_content.strip(), 
            "metadata_tag_original_string": metadata_tag_original_string 
        }

    match_kernel = re.search(r"\[kernel\]\s*(.*)", line_content)
    if match_kernel:
        message = match_kernel.group(1)
        ts_match = re.search(r"at\s*(\d+)ms", line_content)
        timestamp_ms = int(ts_match.group(1)) if ts_match else -1
        hart_match = re.search(r"HART(\d+)", line_content) 
        hart_id = int(hart_match.group(1)) if hart_match else None
        return {
            "type": "kernel",
            "hart": hart_id,
            "tid": None, 
            "time": timestamp_ms,
            "message": message.strip(),
            "original_line": line_content.strip()
        }
    
    ts_match_fallback = re.search(r"at\s*(\d+)ms", line_content)
    timestamp_ms_fallback = int(ts_match_fallback.group(1)) if ts_match_fallback else -1
    hart_match_fallback = re.search(r"HART(\d+)", line_content)
    hart_id_fallback = int(hart_match_fallback.group(1)) if hart_match_fallback else None
    
    return {
        "type": "unknown",
        "hart": hart_id_fallback,
        "tid": None,
        "time": timestamp_ms_fallback,
        "message": line_content.strip(), 
        "original_line": line_content.strip()
    }

def analyze_log_tid_blocks_final(log_content):
    """
    分析日志行，转换通用日志的前缀，并将其格式化为带编号的、
    基于连续TID和HART的块，同时保留层级缩进和ANSI码。
    """
    parsed_lines = []
    for line_num, original_line_text_from_file in enumerate(log_content.splitlines()):
        if not original_line_text_from_file.strip(): 
            continue
        parsed = parse_log_line(original_line_text_from_file) 
        parsed["line_num"] = line_num 
        parsed_lines.append(parsed)

    output_lines = []
    task_indent = {} 
    task_parent = {} 

    current_block_tid = None
    current_block_hart = None
    block_content_indent = "  " 
    block_counter = 0 # 初始化块编号计数器

    for parsed_line in parsed_lines:
        line_tid = parsed_line.get("tid")
        line_hart = parsed_line.get("hart")
        line_msg_for_parent_check = parsed_line.get("message", "") 
        original_full_line_stripped = parsed_line.get("original_line", "")
        line_type = parsed_line.get("type")

        display_line_content = ""
        if line_type == "general":
            original_metadata_tag = parsed_line.get("metadata_tag_original_string") 
            level_val = parsed_line.get("level", "").upper() 
            time_val = parsed_line.get("time") 
            
            new_metadata_replacement = f"[{level_val:>{5}} at {time_val}ms] "
            
            if original_metadata_tag:
                display_line_content = original_full_line_stripped.replace(
                    original_metadata_tag, 
                    new_metadata_replacement, 
                    1 
                )
            else: 
                display_line_content = f"{new_metadata_replacement}{parsed_line.get('message','')}"
        else: 
            display_line_content = original_full_line_stripped

        hierarchical_indent_level = 0
        if line_tid is not None: 
            if line_tid not in task_indent: 
                parent_tid_of_current = task_parent.get(line_tid)
                if parent_tid_of_current and parent_tid_of_current in task_indent:
                    task_indent[line_tid] = task_indent[parent_tid_of_current] + 1
                else:
                    task_indent[line_tid] = 0 
            hierarchical_indent_level = task_indent[line_tid]
        
        hierarchical_indent_str = "  " * hierarchical_indent_level

        if line_tid is not None and line_hart is not None: 
            if line_tid != current_block_tid or line_hart != current_block_hart:
                if current_block_tid is not None and current_block_hart is not None:
                    output_lines.append(f"[TID {current_block_tid}, HART {current_block_hart}]\n")
                
                # 在开始标记中添加块编号
                output_lines.append(f"[#{block_counter} TID {line_tid}, HART {line_hart}]\n")
                block_counter += 1 # 块编号递增
                current_block_tid = line_tid
                current_block_hart = line_hart
            
            output_lines.append(f"{block_content_indent}{hierarchical_indent_str}{display_line_content}\n")

        else: 
            if current_block_tid is not None and current_block_hart is not None:
                output_lines.append(f"[TID {current_block_tid}, HART {current_block_hart}]\n")
                current_block_tid = None 
                current_block_hart = None
            
            output_lines.append(f"{display_line_content}\n")

        if line_tid is not None: 
            m_clone_out = re.search(r"\[syscall\(out\)\]\s*id:\s*SYS_CLONE,\s*res:\s*Ok\((\d+)\)", line_msg_for_parent_check) 
            if m_clone_out:
                child_tid_from_clone = int(m_clone_out.group(1))
                parent_tid_for_clone = line_tid 
                task_parent[child_tid_from_clone] = parent_tid_for_clone
            
            m_spawn_utask = re.search(r"\[spawn_utask\]\s*new task tid\s*=\s*(\d+)", line_msg_for_parent_check) 
            if m_spawn_utask:
                child_tid_from_spawn = int(m_spawn_utask.group(1))
                parent_tid_for_spawn = line_tid 
                task_parent[child_tid_from_spawn] = parent_tid_for_spawn

    if current_block_tid is not None and current_block_hart is not None:
        output_lines.append(f"[TID {current_block_tid}, HART {current_block_hart}]\n")

    return "".join(output_lines)


# --- 主执行逻辑 ---
if __name__ == "__main__":
    # 创建参数解析器
    parser = argparse.ArgumentParser(description="优化日志文件")
    
    # 添加输入文件参数 (位置参数，必需)
    parser.add_argument("-i", "--input_file", help="需要处理的日志文件路径。")
    
    # 添加输出文件参数 (可选参数，有默认值)
    parser.add_argument("-o", "--output_file", 
                        default="./log/out.log", 
                        help="保存处理后日志文件的路径 (默认值: out.log)。")
    
    # 解析命令行参数
    args = parser.parse_args()
    
    # 使用解析得到的参数
    input_file_path = args.input_file
    output_file_path = args.output_file

    try:
        with open(input_file_path, 'r', encoding='utf-8', errors='replace') as f:
            log_content_for_script = f.read()
        
        print(f"正在处理日志文件: {input_file_path}...")
        processed_log_output = analyze_log_tid_blocks_final(log_content_for_script)
        
        with open(output_file_path, "w", encoding='utf-8') as outfile:
            outfile.write(processed_log_output)
        
        print(f"日志分析完成。输出已写入 {output_file_path}")

    except FileNotFoundError:
        print(f"错误: 输入文件 '{input_file_path}' 未找到。")
    except Exception as e:
        print(f"处理过程中发生错误: {str(e)}")