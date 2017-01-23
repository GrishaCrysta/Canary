
Canary
======

Canary is a kernel and operating system written in [Rust](https://rust-lang.org), a new systems programming language that targets safety (a big plus for a kernel). Currently, it only supports the x86-64 architecture, but support for more is planned.

Canary's design goals are:

* **Beauty and simplicity:** in terms of both the code and the graphical user interface and user experience, Canary aims to be both beautiful and simple.
* **Functionality:** we also attempt to balance simplicity with the need for a range of important features like process scheduling, memory management, file systems, and external device support (like keyboards and displays).

## Usage

You can download the latest `.iso` file from the [releases](https://github.com/GravityScore/Canary/releases) page and run it using [VirtualBox](https://www.virtualbox.org/wiki/Downloads) or [QEMU](http://wiki.qemu.org/Main_Page).

For example, using QEMU on an `x86-64` system:

```bash
$ qemu-system-x86_64 -cdrom canary-x86_64.iso
```

## Building

Canary can only be built on 64 bit Linux at the moment. If you're not on Linux, you can use [Vagrant](https://www.vagrantup.com/) (see below).

### Linux

Building Canary requires the following dependencies (keep reading to find out how to install them):

* **[NASM](http://www.nasm.us/):** an x86 assembler.
* **[Nightly Rust](https://doc.rust-lang.org/book/nightly-rust.html):** a nightly build of the Rust compiler is needed in order to use some unstable features required for OS development. We recommend [Rustup](https://rustup.rs/) to manage various Rust installations.
* **[Xargo](https://github.com/japaric/xargo):** manages cross compiling the `core` and `std` crates for Rust.
* **[GRUB Bootloader](https://www.gnu.org/software/grub/):** a multiboot compliant bootloader used by Canary. The `grub-mkrescue` command is used by Canary's build system to generate the `.iso` file.

You can use this `apt-get` command to install NASM:

```bash
$ sudo apt-get install nasm
```

You can use this command to install Rustup (given on the [Rustup homepage](https://rustup.rs/)):

```bash
$ curl https://sh.rustup.rs -sSf | sh
```

Use this command to install `xargo`:

```bash
$ cargo install xargo
$ rustup component add rust-src
```

Next, clone this repository:

```bash
$ git clone https://github.com/GravityScore/Canary
$ cd Canary
```

If you're using Rustup, you need to use the nightly Rust compiler, which you can install and use just for Canary using:

```bash
$ rustup override set nightly
```

Then use the Makefile to build Canary:

```bash
$ make
```

Additionally, if you install [QEMU](http://wiki.qemu.org/Main_Page), you can run Canary easily using:

```bash
$ make run
```

## Other OSes

You'll need to install [Vagrant](https://www.vagrantup.com/). There's a good tutorial for OSX [here](http://sourabhbajaj.com/mac-setup/Vagrant/README.html) which uses [Homebrew](http://brew.sh/). Use these two commands:

```bash
$ brew cask install virtualbox
$ brew cask install vagrant
```

Next, clone this repository:

```bash
$ git clone https://github.com/GravityScore/Canary
$ cd Canary
```

Then start a Linux box (this uses the `Vagrantfile` which comes with the repository) and SSH into it:

```bash
$ vagrant up
$ vagrant ssh
```

Then `cd` into the Canary repository (shared under `/vagrant`):

```bash
$ cd /vagrant
```

Then install the various dependencies required (listed above). NASM can be installed with:

```bash
$ sudo apt-get install nasm
```

And Rustup with:

```bash
$ curl https://sh.rustup.rs -sSf | sh
```

And the nightly Rust compiler using:

```bash
$ rustup override set nightly
```

And `xargo` with:

```bash
$ cargo install xargo
$ rustup component add rust-src
```

Then use the Makefile to build Canary:

```bash
$ make
```

To run Canary, you need to switch back to your host OS (ie. Windows or Mac) and install [QEMU](http://wiki.qemu.org/Main_Page). You can then run Canary using:

```bash
$ make run
```
