/*
 * Copyright (c) 2024 Your Name
 * SPDX-License-Identifier: Apache-2.0
 */

`default_nettype none

// NOTE! This is assuming the 16/8 configuration for maximum ingress
// of f*4/f MB/s (eg. 200 MB/s in, 50 MB/s at 50 MHz)
// Inputs: (uio_in << 8) | ui_in, DDR encoded, 32b per cycle
// Outputs: uo_out, SDR encoded, 8b per cycle

module tt_um_tommythorn_maxbw (
    input  wire [7:0] ui_in,    // Dedicated inputs
    output wire [7:0] uo_out,   // Dedicated outputs
    input  wire [7:0] uio_in,   // IOs: Input path
    output wire [7:0] uio_out,  // IOs: Output path
    output wire [7:0] uio_oe,   // IOs: Enable path (active high: 0=input, 1=output)
    input  wire       ena,      // always 1 when the design is powered, so you can ignore it
    input  wire       clk,      // clock
    input  wire       rst_n     // reset_n - low to reset
);

  // All output pins must be assigned. If not used, assign to 0.
  assign uio_out = 0;
  assign uio_oe  = 0; // uio is configured as inputs

  // List all unused inputs to prevent warnings
  wire _unused = &{ena, clk};

  // DDR input: two sample flops
  // Low part sampled on rising edge, high part on falling edge
  reg [15:0]	      in_lo, in_hi;
  reg [ 7:0]	      out;
  always @(posedge clk) in_lo <= {uio_in, ui_in};
  always @(negedge clk) in_hi <= {uio_in, ui_in};

  // SDR outputs
  assign uo_out = out;
  always @(posedge clk)
    if (rst_n == 0)
      // This behavior is only available whilst in reset
      out <= in_hi[15:8] ^ in_hi[7:0] ^ in_lo[15:8] ^ in_lo[7:0];  // out = F(in)
    else
      out <= 0;

   initial
     $monitor("%05d  clk %d rst# %d  in %x,%x  out %x   in_lo %x in_hi %x",
	      $time,
	      clk, rst_n, uio_in, ui_in, uo_out,
	      in_lo, in_hi);
endmodule
