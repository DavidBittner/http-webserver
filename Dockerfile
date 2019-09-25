FROM rust:latest

ENV listen_port 80
ENV server_home /home/http-server
ENV test_url https://raw.githubusercontent.com/ibnesayeed/webserver-tester/master/sample/cs531-test-files.tar.gz
ENV server_root /tmp/

RUN mkdir -p ${server_root}
ADD ${test_url} ${server_root}/test_files.tar.gz
RUN cd ${server_root} && tar -xf ${server_root}/test_files.tar.gz
RUN rm ${server_root}/test_files.tar.gz

RUN apt -y update
RUN apt -y install tree
RUN tree ${server_root}

EXPOSE  ${listen_port}
WORKDIR ${server_home}

RUN mkdir -p ${server_home}

COPY src ${server_home}/src
COPY Cargo.toml ${server_home}/
COPY config.yml ${server_home}/

RUN cd ${server_home}
RUN cargo build --release
RUN echo "#!/bin/sh \n RUST_LOG=trace SERV_ROOT=/tmp/ SERV_PORT="${listen_port}" ./target/release/http-webserver" > ${server_home}/start.sh
RUN chmod uo+x ${server_home}/start.sh

ENTRYPOINT ["/home/http-server/start.sh"]
