/*
 * Copyright (c) 2024 Your Name
 * SPDX-License-Identifier: Apache-2.0
 */

`default_nettype none

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
  assign uio_oe  = 0;

  // List all unused inputs to prevent warnings
  wire _unused = &{ena, clk, rst_n, 1'b0};

  // Rolling my own [untested] DDR input: two sample flops + one synchronizing
  reg [15:0]	      in_lo, in_hi, in_lo_r;
  reg [ 7:0]	      out;

  assign uo_out = out;

  always @(negedge clk) in_lo <= {uio_in, ui_in};
  always @(posedge clk) in_lo_r <= in_lo;
  always @(posedge clk) in_hi <= {uio_in, ui_in};
  always @(posedge clk) out <= in_hi[15:8] ^ in_hi[7:0] ^ in_lo_r[15:8] ^ in_lo_r[7:0];  // out = F(in)
endmodule
