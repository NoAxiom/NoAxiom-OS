# 如果不存在镜像文件，则进行复制
# 如果存在镜像文件，则进行哈希值对比，若不一致也进行复制
import os
import shutil
import argparse
import logging
import hashlib
import multiprocessing

class LogColors:
    INFO = "\033[92m"  # Green
    WARNING = "\033[93m"  # Yellow
    ERROR = "\033[91m"  # Red
    RESET = "\033[0m"  # Reset to default

class ColoredFormatter(logging.Formatter):
    def format(self, record):
        if record.levelno == logging.INFO:
            record.msg = f"{LogColors.INFO}{record.msg}{LogColors.RESET}"
        elif record.levelno == logging.WARNING:
            record.msg = f"{LogColors.WARNING}{record.msg}{LogColors.RESET}"
        elif record.levelno == logging.ERROR:
            record.msg = f"{LogColors.ERROR}{record.msg}{LogColors.RESET}"
        return super().format(record)

def hash_file_blake2b(filepath, chunk_size=8192):
    """计算文件的 BLAKE2b 哈希值。"""
    hasher = hashlib.blake2b()
    try:
        with open(filepath, 'rb') as f:
            # 仅检查前 1 / 30 的文件大小
            count = 524288 // 30
            while True:
                chunk = f.read(chunk_size)
                if count == 0:
                    break
                count -= 1
                hasher.update(chunk)
        return hasher.hexdigest()
    except FileNotFoundError:
        return None
    except Exception as e:
        print(f"Error hashing file {filepath} with BLAKE2b: {e}")
        return None

def check_or_copy(src_path, dest_path):
    logging.info(f"Checking image between {src_path} and {dest_path}...")
    if os.path.exists(dest_path):
        src_hash = hash_file_blake2b(src_path)
        dest_hash = hash_file_blake2b(dest_path)
        
        if src_hash == dest_hash:
            logging.info(f"Image is clean! No action taken.")
            return
        else:
            logging.info(f"Image hash mismatch! Copying new image to {dest_path}.")
    else:
        logging.info(f"Image does not exist at {dest_path}! Copying new image to {dest_path}.")
    
    shutil.copy2(src_path, dest_path)
    logging.info(f"Copied image from {src_path} to {dest_path}.")

def copy_image(src_path, dest_path):
    shutil.copy2(src_path, dest_path)

if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    handler = logging.StreamHandler()
    handler.setFormatter(ColoredFormatter("%(levelname)s: %(message)s"))
    logging.getLogger().handlers = [handler]

    parser = argparse.ArgumentParser(description="Makefile Python helper script.")
    # 创建子解析器，每个子解析器对应一个可以从 Makefile 调用的函数
    subparsers = parser.add_subparsers(dest="function_name", help="Function to execute", required=True)

    check_or_copy_parser = subparsers.add_parser("check_or_copy", help="Check the image file and copy if necessary.")
    check_or_copy_parser.add_argument("--src", help="Source image path.")
    check_or_copy_parser.add_argument("--dest", help="Destination image path.")
    check_or_copy_parser.set_defaults(func=check_or_copy)

    copy_parser = subparsers.add_parser("copy_image", help="Copy the image file.")
    copy_parser.add_argument("--src", help="Source image path.")
    copy_parser.add_argument("--dest", help="Destination image path.")
    copy_parser.set_defaults(func=copy_image)

    args = parser.parse_args()

    if args.function_name == "check_or_copy":
        args.func(src_path=args.src, dest_path=args.dest)
    elif args.function_name == "copy_image":
        args.func(src_path=args.src, dest_path=args.dest)
    else:
        parser.print_help()