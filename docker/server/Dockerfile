FROM ubuntu:18.04

MAINTAINER Jonas Deyson <jonasdeyson@gmail.com>
EXPOSE 8096/tcp

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y rsync locales

# Set the locale
RUN locale-gen en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8

# install fbksd-ci app
COPY fbksd-server /usr/bin/
RUN chmod +x /usr/bin/fbksd-server

# NOTE: This container must be executed using the user fbksd from the host.
RUN groupadd -g 992 -r fbksd-ci
RUN useradd -r -u 993 -g fbksd-ci --no-create-home fbksd-ci
RUN groupadd -g 991 -r fbksd
RUN useradd -r -u 992 -g fbksd -G fbksd-ci --no-create-home fbksd
#USER fbksd
CMD fbksd-server
