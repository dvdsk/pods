# pods
This is a podcast app in early development. It was created to fill the void of mobile linux podcast apps. Right now you can use it for very basic podcast listening. The following is working (on mobian):

- Adding a podcast by entering an rss feed link
- Searching for a podcast by name, then adding by clicking a result
- New episodes are added on startup
- Streaming (play during download) episodes
- Download episode then play
- Resume from last position when playing again
- Skip forward and backward

## Large limitations
For other issues see the issues tab.

- Does not work on manjaro on the Pinephone
- Crashes whenever an unimplemented feature is used
	- dont try the rm button

## Compile and Setup
How to compile and setup on the PinePhone running Mobian. There are two options,

### compile on the phone 
1. install [rust](https://www.rust-lang.org/learn/get-started) 
2. install required packages/libs on your device
	- pkgconf, (apt: pkg-config
	- libasound2-dev, (apt: libasound2-dev)
	- libx11-dev, (apt:libx11-dev)
3. clone this repo somewhere cd into it
4. run `cargo build --release`
you can now copy the binary from: `target/release/pods`

### crosscompile from desktop
0. install docker
1. install [rust](https://www.rust-lang.org/learn/get-started) 
2. install rust cross using: `cargo install cross`
3. clone this repo somewhere cd into it
4. run `./crosscomp.sh`
you can now copy the binary from: `target/aarch64-unknown-linux-gnu/release/pods`

### setup
I use and test on mobian with Phosh at the moment, below assumes you are running mobian with the default user name `mobian`.
1. make the directory `/home/mobian/bin`
2. copy the binary to `/home/mobian/bin/pods`
3. copy `icon.png` and `start_pods.sh` from the repository to `/home/mobian/bin`
3. copy `pods.desktop` from the repository to `/home/mobian/.local/share/applications/`
4. either reboot or run `gtk-update-icon-cache`
There should now be an app called pods that you can launch now

## Usage
How to use the app:
First you need to add a podcast: either search by name (press enter to get results) or paste in in the rss feed url. 
- Click the podcast name to view the episodes
- Click an episode to play it, if it was not downloaded this will "stream"
- Download an episode by clicking the `dl` button
- "Scroll" through the podcast list using the up and down button
- pause and resume using the `Resume` button
- Skip 5 seconds forward or backward using `fwd` and `bck`

## Issues
For now there are a lot of issues. This app will get more stable over time. Feel free to open an issue, I do not recommend trying to fix things for now, the code will undergo a lot of refactoring.
