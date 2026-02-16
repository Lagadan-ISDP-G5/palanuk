# Final major refactor

To minimize blast radius we'll keep the normalized speed semantics (0.0 to 1.0). Except we'll map
it from 0 rpm to MAX rpm, where MAX is a constant.

Right before the cu-propulsion task and after the arbitrator task, we will have another GenericPIDTask
responsible for controlling the actual speed of the motors by varying the motors' duty cycles.
This task is not runtime-overridable, it will run even in open loop mode.
