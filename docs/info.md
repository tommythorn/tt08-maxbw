<!---

This file is used to generate your project datasheet. Please fill in the information below and delete any unused
sections.

You can also include images in this folder and reference them in the markdown. Each image must be less than
512 kb in size, and the combined size of all images must be less than 1 MB.
-->

## How it works

All bidirectional IOs are used as inputs.  The total 16 inputs are
sampled at both clock edges (DDR).  Every posedge of clock, the
resulting 32-bit are reduced to 8-bit (some xor combination).

## How to test

Drive it with a very dedicated test bench (TBD).

## External hardware

List external hardware used in your project (e.g. PMOD, LED display, etc), if any
