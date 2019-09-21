FROM rust:latest

ENV listen_port 80
ENV server_home /home/http-server

EXPOSE ${listen_port}

WORKDIR ${server_home}

RUN mkdir -p ${server_home}

COPY src ${server_home}/src
COPY Cargo.toml ${server_home}/

RUN cd ${server_home}
RUN cargo build --release
RUN echo "#!/bin/sh \n RUST_LOG=trace HTTP_PORT="${listen_port}" ./target/release/echo-server" > ${server_home}/start.sh
RUN chmod uo+x ${server_home}/start.sh

ENTRYPOINT ["/home/http-server/start.sh"]
