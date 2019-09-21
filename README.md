# CS531 Web Server

This is a webserver written in Rust for my CS531 class at ODU. Once the course is completed it will be open-sourced.

# Structure

The structure of this project consists of separate crates within the root crate. The root crate is responsibile for generating the actual webserver binary, whereas the child crates are simply libs the parent crate uses.

# Dependencies

There are intentionally few dependencies. All parsing will be done using the stdlib (not using a lib such as nom).
