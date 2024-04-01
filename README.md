Colorlight HDMI
===============

RootFS for the surtitles screen, doing HDMI to colorlight protocol

# Compiling the rootfs

1. Clone this repository with its submodules
1. Go to buildroot folder
1. Load config with `make BR2_EXTERNAL=$(realpath ../external) odroidc1_defconfig`
1. Compile the OS with `make`
1. Have a coffee, and play fetch with your dog, if you don't have one, just look on youtube Husky's videos
1. This WILL take a while
1. Congratulations, your rootfs is available on `output/images/rootfs.tar`!
