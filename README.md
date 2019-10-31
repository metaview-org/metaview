# Compiling

Install dependencies required by Monado. Those include:
```
sudo apt install libeigen3-dev libv4l-dev libglew-dev libusb-1.0-0-dev libhidapi-dev libxcb1-dev libxcb-randr0-dev
```

Additionally, one must compile and install the following projects:
* OpenHMD
* glslang

In order for Monado to see HTC Vive, install special udev rules:
```
sudo apt install steam-devices libudev-dev
```
Make sure to restart the computer to apply these changes.
