use alloc::{string::String, vec::Vec};

use include::{errno::Errno, return_errno};

use super::{Syscall, SyscallResult};
use crate::{mm::user_ptr::UserPtr, panic::kshutdown, utils::random_fill};

impl Syscall<'_> {
    /// get a random number
    pub async fn sys_getrandom(&self, buf: usize, buflen: usize, _flags: usize) -> SyscallResult {
        info!("[sys_getrandom] buf: {:#x}, buflen: {}", buf, buflen);
        let user_ptr = UserPtr::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;
        random_fill(buf_slice);
        Ok(buflen as isize)
    }

    /// systemshutdown
    pub fn sys_systemshutdown() -> ! {
        println!("[kernel] system shutdown (syscall)");
        kshutdown()
    }

    /// add_key() creates or updates a key of the given type and
    /// description, instantiates it with the payload of size size,
    /// attaches it to the nominated keyring, and returns the key's serial
    /// number.
    /// keyutils.h
    /// key_serial_t add_key(const char *type, const char *description,
    // const void payload[.size], size_t size,
    // key_serial_t keyring);
    pub async fn sys_add_key(
        &self,
        type_ptr: usize,
        description_ptr: usize,
        payload_ptr: usize,
        payload_size: usize,
        keyring: usize,
    ) -> SyscallResult {
        warn!("syscall sys_add_key get called, which is not well implemented yet");
        let keytype = UserPtr::new(type_ptr)
            .get_string_from_ptr()
            .map_err(|_| Errno::EFAULT)?;
        let description = UserPtr::new(description_ptr)
            .get_string_from_ptr()
            .map_err(|_| Errno::EFAULT)?;
        let payload_enabled = match keytype.as_str() {
            "keyring" => {
                if payload_size != 0 {
                    return_errno!(Errno::EINVAL);
                }
                false
            }
            "user" => {
                if payload_size > 32767 {
                    return_errno!(Errno::EINVAL);
                }
                true
            }
            "logon" => {
                if payload_size > 32767 {
                    return_errno!(Errno::EINVAL);
                }
                true
            }
            "big_key" => {
                if payload_size > (1 << 20) - 1 {
                    return_errno!(Errno::EINVAL);
                }
                true
            }
            "asymmetric" | "cifs.idmap" | "cifs.spnego" | "pkcs7_test" | "rxrpc" | "rxrpc_s" => {
                true
            }
            _ => {
                return_errno!(Errno::EINVAL);
            }
        };
        if payload_enabled {
            let payload_slice = Vec::from(
                UserPtr::<u8>::new(payload_ptr)
                    .as_slice_const_checked(payload_size)
                    .await
                    .map_err(|_| Errno::EFAULT)?,
            );
            // let _payload = unsafe {
            // String::from_utf8_unchecked(payload_slice) };
        }
        Ok(0)
    }
}
