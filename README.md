
# pods

## warning: still pre-alpha/early development

This is a podcast app in early development. It was created to fill the void of mobile linux podcast apps. Right now you can use it for very basic podcast listening. The following is working on [Mobian][mobian]:

- Adding a podcast by entering an rss feed link
- Searching for a podcast by name, then adding by clicking a result
- New episodes are added on startup
- Streaming (play during download) episodes
- Download episode then play
- Resume from last position when playing again
- Skip forward and backward

## Large Limitations

For other issues see the issues tab.

- Does not work on manjaro on the Pinephone
- Crashes whenever an unimplemented feature is used
  - Don't try the `rm` button

## Compile and Setup

To compile and setup on the PinePhone running Mobian. There are two options:

### Install Dependencies

The following libraries must be available on the system:

- libfreetype2
- pkgconf
- libasound2-dev
- libx11-dev

On Debian-based systems (this includes Ubuntu and Linux Mint among others) you can use this command:

```terminal
sudo apt install \
  gcc \
  g++ \
  cmake \
  libx11-dev
  pkg-config \
  libasound2-dev \
  libfreetype-dev \
  libexpat1-dev \
```

### Compile on the Phone 

1. Install [Rust][rust] 
2. Install the required packages/libs on your device (see above)
3. Clone this repo somewhere `cd` into it
4. Cun `cargo build --release`

You can now copy the binary from: `target/release/pods`

### Crosscompile from Desktop

0. Install docker
1. Install [Rust][rust] 
2. Install Rust cross using: `cargo install cross`
3. Clone this repo somewhere `cd` into it
4. Run `./crosscomp.sh --release`

You can now copy the binary from: `target/aarch64-unknown-linux-gnu/release/pods`

### Setup

I use and test on Mobian with Phosh at the moment. The following instructions assume you are running Mobian with the default user name `mobian`:

1. make the directory `/home/mobian/bin`
2. copy the binary to `/home/mobian/bin/pods`
3. copy `icon.png` and `start_pods.sh` from the repository to `/home/mobian/bin`
4. copy `pods.desktop` from the repository to `/home/mobian/.local/share/applications/`
5. either reboot or run `gtk-update-icon-cache`
There should now be an app called pods that you can launch now


## How to use the App

First you need to add a podcast with the text field at the top of the screen: either search by name (press enter to get results) or directly paste the rss feed url. 

- Click the podcast name to view the episodes
- Click an episode to play it. If it was not downloaded befor this will "stream" it.
- Download an episode by clicking the `dl` button
- "Scroll" through the podcast list using the up and down button
- Pause and resume using the `Resume` button
- Skip 5 seconds forward or backward using the `fwd` and `bck` buttons

## Issues

For now there are a lot of issues. This app will get more stable over time. Feel free to open an issue, I do not recommend trying to fix things for now, the code will undergo a lot of refactoring.

[mobian]: https://mobian-project.org
[rust]: https://www.rust-lang.org/learn/get-started
