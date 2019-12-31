FROM nvidia/cuda:10.1-devel

MAINTAINER Jonas Deyson <jonasdeyson@gmail.com>

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
  build-essential git g++-6  \
  rsync libopencv-dev qt5-default  \
  libboost-all-dev libopenexr-dev  \
  libeigen3-dev libxerces-c-dev libfftw3-dev \
  libglewmx-dev curl wget locales

# Set the locale
RUN locale-gen en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8

# a more recent cmake version was needed to fix a CUDA 10.1 error with LWR.
RUN wget https://github.com/Kitware/CMake/releases/download/v3.14.1/cmake-3.14.1-Linux-x86_64.sh
RUN sh cmake-3.14.1-Linux-x86_64.sh --skip-license --prefix=/usr/

# install fbksd core
WORKDIR /tmp
RUN git clone --recurse-submodules https://github.com/fbksd/fbksd.git
WORKDIR fbksd
RUN mkdir build && cd build && cmake -DCMAKE_BUILD_TYPE=Release ../ && make install && ldconfig

# install utilities
COPY fbksd-ci /usr/bin/
RUN chmod +x /usr/bin/fbksd-ci

RUN groupadd -g 992 -r fbksd-ci
RUN useradd -r -u 993 -g fbksd-ci --no-create-home fbksd-ci
USER fbksd-ci

