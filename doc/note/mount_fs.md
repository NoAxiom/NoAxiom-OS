mount

```shell
sudo losetup loop0 fs.img
sudo mkdir /mnt/fs_img
cd /mnt/fs_img
sudo mount /dev/loop0 /mnt/fs_img 
```

unmount
```shell
sudo umount /mnt/fs_img
sudo losetup -d /dev/loop0
```