# virst-revive
[virt-revive](https://github.com/jacksonsteiner/virt-revive) rewritten in Rust.

Refactored from C++ because of guaranteed memory safety, easier dependency management, and to learn a new language.

Manage a QEMU/KVM virtual machine that needs to be in an always running state. First defines the domain if not already defined, then keeps the virtual machine running and restarts it if it were to shutdown. Resumes the virtual machine if it gets suspended/paused, and will try to restart the virtual machine on a crash. Attempts to gracefully shutdown the virtual machine on signal interrupt receive, but will destroy the domain after a 10 second timeout while keeping the domain defined (and the xml file intact).

Build with:

    git clone https://github.com/jacksonsteiner/virst-revive.git
    cd virst-revive
    cargo build --release


Provide the path to the xml domain to be managed.

Run with:

    ./target/release/virst-revive /absolute/path/to/domain/xml.xml

