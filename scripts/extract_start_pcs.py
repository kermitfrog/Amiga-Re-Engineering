#!/usr/bin/python
import os
import sys

# Extracting starting pcs is actually better done with dump-analyzer p now
# This script is kept, because of the PC history file it creates
PC = 1

dump_dir = sys.argv[1]
if len(sys.argv) > 2:
    max_count = sys.argv[2]
else:
    max_count = 1

if not os.path.isdir(dump_dir):
    exit(1)

if not os.path.isfile(dump_dir + '/hist'):
    os.system("ack -B1 \'Next PC:\' " + dump_dir + "/opcode.log | ack -v \'Next PC:\' | ack -v \'^--$\' > " + dump_dir + "/pcs")
    os.system("cut -f1 -d\\  " + dump_dir + "/pcs | sort | uniq -c | sort -n > " + dump_dir + "/hist")

with open(dump_dir + "/hist") as hist:
    line = hist.readline().split()
    pc_start = int(line[PC], 16)
    pc_last = pc_start
    while True:
        line = hist.readline().split()
        if int(line[0]) > max_count:
            exit(0)

        pc_new = int(line[PC], 16)
        if pc_new > pc_last + 10:
            print("{0:0>8X}".format(pc_start))
            pc_start = pc_new
        pc_last = pc_new





