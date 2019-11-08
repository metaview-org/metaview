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

In order to get the Monado `experimental/libsurvive` branch working, which
provides 6DOF tracking, the following libraries must be installed:
```
sudo apt install libpcap-dev
sudo apt-get install liblapacke-dev libopenblas-dev libatlas-base-dev
sudo apt-get install build-essential zlib1g-dev libx11-dev libusb-1.0-0-dev freeglut3-dev
```
