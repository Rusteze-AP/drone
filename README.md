# Rust-eze drone specifications

This is the repository containing the Drone provided by Rust-eze group for the Advanced Programming course (a.y. 2024/2025).

## Importing the project

Add our crate to your `Cargo.toml` file:

```toml
rusteze_drone = { git = "https://github.com/Rusteze-AP/drone.git", branch = "main" }
```

Each group that bought the drone will have access to a telegram group where we communicate updates and changes to the drone (which will require `cargo update`). If you are not part of the group, please contact us.

## Usage

Add to any of your files `*.rs`:
    
```rust
use rusteze_drone::RustezeDrone;
```

## Logging

The drone **by default** does not log anything. If you want to enable the logging, you can enable only some levels or all of them, **before running it**. To do so, you can use the following functions:

```rust
use rusteze_drone::RustezeDrone;

fn main() {
    let drone = RustezeDrone::new(...);

    // Enable which levels you want to log
    drone.with_all(); // Enable all levels
    drone.with_debug(); // Enables only debug level
    drone.with_info(); // Enables only info level
    drone.with_warn(); // Enables only warn level
    drone.with_error(); // Enables only error level

    // Forward the drone logs to a websocket
    drone.with_websocket();

    // Start the drone
    drone.run();
}
```

To connect to the WebSocket, if enabled through the `with_websocket` method, you can use the following command in the terminal:

```bash
wscat -c ws://127.0.0.1:3030/ws
```

Alternatively, you can use your preferred library to connect to a WebSocket at the address `ws://127.0.0.1:3030/ws`.

## Tests

Tests can be found in the corresponding repository, available [here](https://github.com/Rusteze-AP/rusteze-tests) and can be imported and used by any group. 

Additionally, you can find them in the `tests` folder of this repository and use them by running `./run_tests.sh`.

## Support

- Telegram group (link sent privately)
- Create an issue on this repository