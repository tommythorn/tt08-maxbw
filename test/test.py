# SPDX-FileCopyrightText: © 2024 Tiny Tapeout
# SPDX-License-Identifier: Apache-2.0

import cocotb
from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge, FallingEdge


@cocotb.test()
async def test_project(dut):
    dut._log.info("Start")

    # Set the clock period to 15 ns (66 MHz)
    clock = Clock(dut.clk, 15, units="ns")
    cocotb.start_soon(clock.start())

    # Reset
    dut._log.info("Reset -- Test bit echo mode")
    dut.rst_n.value = 0
    dut.ena.value = 1
    dut.ui_in.value = 0
    dut.uio_in.value = 0
    await ClockCycles(dut.clk, 5)
    assert dut.uo_out.value == 0

    # We clock in a pattern and expect the xor back
    await FallingEdge(dut.clk)
    dut.ui_in.value = 1977 & 255;
    dut.uio_in.value = 1977 >> 8;
    await RisingEdge(dut.clk)
    dut.ui_in.value = 9724 & 255;
    dut.uio_in.value = 9724 >> 8;
    await FallingEdge(dut.clk)
    dut.ui_in.value = 0 # idle at zero
    dut.uio_in.value = 0 # idle at zero

    await RisingEdge(dut.clk)
    await FallingEdge(dut.clk)

    assert dut.uo_out.value == ((1977 ^ 9724 ^ ((1977 ^ 9724) >> 8)) & 255)

    await RisingEdge(dut.clk)
    await FallingEdge(dut.clk)

    assert dut.uo_out.value == 0

    dut._log.info("Test Reset Transport behavior")

    # XXX Should wait for a command first
    dut.rst_n.value = 1
    await ClockCycles(dut.clk, 2)

    # Begin reply, DDR encoded data transfer on rising edges
    await FallingEdge(dut.clk)

    # Send packet header; currently just the size is (3+1)*2 = 8B
    dut.ui_in.value = 3
    await RisingEdge(dut.clk)
    dut.ui_in.value = 11
    await FallingEdge(dut.clk)
    dut.ui_in.value = 22
    await RisingEdge(dut.clk)
    dut.ui_in.value = 33
    await FallingEdge(dut.clk)

    # idle channel must always be zero
    dut.ui_in.value = 0


    # The following assersion is just an example of how to check the output values.
    # Change it to match the actual expected output of your module:
    # assert dut.uo_out.value == 50

    # Keep testing the module by changing the input values, waiting for
    # one or more clock cycles, and asserting the expected output values.
