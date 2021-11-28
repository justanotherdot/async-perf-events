# Notes

## Running the test program under `perf`

Running the output binary under `perf stat -ad` seems to cause the counters to
go blank when read in the binary, but the surrounding perf invocation has (full)
counters. I imagine this is because of the perf_event_open and PMU API in that
once a counter is open, it cannot be re-opened until cleared or restored.
