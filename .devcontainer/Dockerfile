# Use the official Amazon Linux image
FROM amazonlinux:latest

ARG USERNAME=amazonlinux
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN yum update -y \
    && yum install -y shadow-utils sudo \
    && yum clean all

# Create the user
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo "$USERNAME ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

# Install development tools and dependencies
RUN yum groupinstall -y "Development Tools" \
    && yum install -y \
    git \
    vim \
    tar \
    gzip \
    openssl-devel \
    perl \
    perl-core \
    perl-IPC-Cmd \
    wget \
    glibc-langpack-en \
    && yum clean all

# Switch to the created user
USER $USERNAME

# Install Rust and Cargo tools as the amazonlinux user
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && ~/.cargo/bin/cargo install cargo-lambda \
    && ~/.cargo/bin/cargo install just \
    && ~/.cargo/bin/cargo install --git https://github.com/fpco/amber \
    && ~/.cargo/bin/cargo install pqrs \
    && ~/.cargo/bin/cargo install --git  https://github.com/helix-editor/helix helix-term --locked \
    && ~/.cargo/bin/rustup install nightly \
    && ~/.cargo/bin/rustup target add x86_64-unknown-linux-musl \
    && ~/.cargo/bin/cargo install cargo-udeps

# Set environment variables
ENV PATH="/home/$USERNAME/.cargo/bin:${PATH}"
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8
ENV OPENSSL_DIR=/usr \
    OPENSSL_LIB_DIR=/usr/lib64 \
    OPENSSL_INCLUDE_DIR=/usr/include \
    CC=gcc

# Set the working directory
WORKDIR /workspace