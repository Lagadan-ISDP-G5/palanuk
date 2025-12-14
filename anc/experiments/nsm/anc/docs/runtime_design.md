## Runtime Design

This is less of a design document and more of a random collection of thoughts against complexity and abstraction.

## Why not ROS

I'd rather be yelled at by the borrow checker than turning limbless from the foot bazookas of C++.

## Why copper-rs

The immediate previous version of this doc argued against `copper-rs`, but I decided to go ahead on using it, at the risk of reinventing the wheel. After some rumination, I foresee that I'll end up worse without
the copper runtime, especially when it comes to debugging. Integrating various peripherals is not a triviality, so I will likely reinvent a worse `copper-rs` anyway that at best will take up as much
time to engineer as `copper-rs` but is completely bespoke and separate from the Rust robotics community.

## Verdict

We're absolutely using `copper-rs`, but only its barely useful features.
