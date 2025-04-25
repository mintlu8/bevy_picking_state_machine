# bevy_picking_state_machine

An opinionated global state machine for `bevy_picking`. This serves as a drop in replacement
of `PickingInteraction`, `ButtonInput<MouseButton>` and `Window::pointer` that can also handle events like observers. Unlike `PickingInteraction` this crate handles global state like dragging correctly.

## Rules

* One action at a time

    Only one entity can be "active", i.e. hovered or pressed.
    Each entity can only receive one event (like `HoverEnter`) in a frame.
    There will be no multi-cursor support due to this assumption.

* Single button only

    The state only tracks one button.
    Pressing multiple buttons is treated as canceling the current click or drag.
    This state persists until all buttons are released.

* Clean interactions

    If any recognized button is already pressed, no new entities can be registered as hovered or pressed.

## Getting Started

Add `PickingStateMachinePlugin`, do your normal `bevy_picking` setup, then use `Res<PickingStateMachine>` in your system over `PickingInteraction`, that's it!

## Versions

| bevy | bevy_picking_state_machine |
|------|----------------------------|
| 0.16 | 0.1-latest                 |

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
