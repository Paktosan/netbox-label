Linux requires udev rule:
```
SUBSYSTEM=="usb", ATTRS{product}=="PT-P910BT", GROUP="users"
```
