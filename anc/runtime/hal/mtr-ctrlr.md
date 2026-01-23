# On the generic `Motor`

It originally came from a design decision to have two separate PID controllers control each motor individually. However to simplify, we decided to first see if a single PID controller would suffice instead, which is what the `dual-mtr-ctrlr` example is for.
