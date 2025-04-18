# bevy_picking_state_machine

An opinionated global state machine for `bevy_picking`. This serves as a more robust version of
`PickingInteraction` that can also handle events

## Rules

* One action at a time

    Only one entity can be "active", i.e. hovered or pressed.

    There is no multi-cursor support.

* Single button only

    The state only tracks one button.
    Pressing multiple buttons is treated as canceling the current click or drag.
    This state persists until all buttons are released.

* Clean interactions

    If any registered button is already pressed, no new entities can be registered as hovered or pressed.
