## Runtime Design

This is less of a design document and more of a random collection of thoughts against complexity and abstraction.

## Why not ROS

I'd rather be yelled at by the borrow checker than turning limbless from the foot bazookas of C++.

## Why not copper-rs

[complexity _very_, _very_, bad.](https://grugbrain.dev)

## Verdict

We'll make our own dumb robotics runtime that scales poorly, has no deterministic replay, and has a large logging overhead.

## Priorities

Minimize LOC, minimize chances of logic bugs, and most importantly minimize engineering time. We're not building a Mars rover, this project is a one-time thing.
