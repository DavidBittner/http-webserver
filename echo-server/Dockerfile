FROM rust:latest

ENV listen_port 80
EXPOSE 8080:${listen_port}/tcp

ENV server_home /home/echo-server

WORKDIR ${server_home}

RUN mkdir -p ${server_home}

COPY src ${server_home}/src
COPY Cargo.toml ${server_home}/

RUN cd ${server_home}
RUN cargo build --release
RUN echo "#!/bin/sh \n ECHO_PORT="${listen_port}" ./target/release/echo-server" > ${server_home}/start.sh
RUN chmod uo+x ${server_home}/start.sh

ENTRYPOINT ["/home/echo-server/start.sh"]
