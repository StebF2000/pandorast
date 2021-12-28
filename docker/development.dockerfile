FROM rust:latest

# Install needed dev packages
RUN apt-get update -y && \
    apt-get install -y build-essential cmake gdb openssh-server \
    valgrind texlive && rm -rf /var/cache/apt/archives

# Default powerline10k theme, no plugins installed
RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v1.1.2/zsh-in-docker.sh)"

# Log in as root user
RUN echo 'root:root' | chpasswd && \
    mkdir /var/run/sshd && \
    echo "PermitRootLogin yes" >> /etc/ssh/sshd_config

EXPOSE 22 63342

# -D flag runs sshd in foreground
CMD ["/usr/sbin/sshd", "-D"]
