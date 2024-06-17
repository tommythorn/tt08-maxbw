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

`ifdef SIM
   initial
     $monitor("%05d  clk %d rst# %d  in %x,%x  out %x   in_lo %x in_hi %x",
	      $time,
	      clk, rst_n, uio_in, ui_in, uo_out,
	      in_lo, in_hi);
`endif
endmodule

// XXX this could be parametrized...

// XXX Currently requiring that DDR packets are a multiple of 32b
// and only starts on posedges.
module deserialize16DDR (
    input  wire        clock,
    input  wire        reset,
    input  wire [15:0] ser_in_ddr,

    output reg         packet_valid,
    output reg         packet_is_header,
    output reg  [31:0] packet,
);

   // DDR input: two sample flops
   // Low part sampled on rising edge, high part on falling edge
   reg [ 7:0]	       out;

   reg [ 4:0]	       packet_words_left;

   // Header | aux:4 | tag:4 | cmd:3 | size:5 |

   always @(posedge clock) in_lo <= ser_in_ddr;
   always @(negedge clock) in_hi <= ser_in_ddr;

   always @(posedge clock) packet_valid     <= !reset && in_lo != 0 || packet_words_left != 0;
   always @(posedge clock) packet_is_header <= !reset && in_lo != 0 && packet_words_left == 0;
   always @(posedge clock) if (reset)
     packet_words_left <= 0;
   else if (packet_words_left != 0)
     // In the middle of a packet with `packet_words_left` of payload left
     packet_words_left <= packet_words_left - 1;
   else if (in_lo != 0 && packet_words_left == 0)
     // We have a new header, collect the size
     packet_words_left <= in_lo[4:0];
   always @(posedge clock) packet <= {in_hi,in_lo};
endmodule

// XXX Currently requiring that DDR packets are a multiple of 32b
// and only starts on posedges.
module serialize8SDR (
    input wire	      clock,
    input wire	      reset,

    input wire	      valid,
    input wire [ 1:0] packet_size,
    input wire [31:0] packet_header,
    input wire [31:0] packet_payload0,
    input wire [31:0] packet_payload1,
    input wire [31:0] packet_payload2,
    input wire [31:0] packet_payload3,

    output wire       ready,
    output reg [7:0]  ser_out_sdr,
);

   reg [ 4:0]	       packet_words_left;

   ....
endmodule

// + deserialize8SDR & serialize16DDR for a test reg
