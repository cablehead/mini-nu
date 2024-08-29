A variation on the [main](https://github.com/cablehead/mini-nu) branch that
runs Nushell's engine standalone, reusing it across threads. For each line of
input, it spawns a new thread to execute the user-provided closure, passing an
incrementing count as an argument.

```
% cargo r '{|arg| let s = $in ; sleep 5sec; $"($in | str upcase) :: ($arg)"}'
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.57s
     Running `target/debug/mini-nu '{|arg| let s = $in ; sleep 5sec; $"($in | str upcase) :: ($arg)"}'`
abc
Thread 0 starting execution
you and me
Thread 1 starting execution
Thread 0: ABC :: 0
Waiting for all tasks to complete...
Thread 1: YOU AND ME :: 1
All tasks completed. Exiting.
```
