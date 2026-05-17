FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    socat \
    libc6-i386 \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /home/q36
WORKDIR /app

COPY esp .
COPY flag.txt /home/q36/flag.txt
RUN chmod +x esp

EXPOSE 10777

CMD ["socat", "TCP-LISTEN:10777,reuseaddr,fork", "EXEC:./esp"]