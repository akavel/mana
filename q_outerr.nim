import os

stdout.writeLine "some stdout"
stderr.writeLine "some stderr"
stderr.writeLine "some stderr2"
stdout.writeLine "some stdout2"
stdout.flushFile()
stderr.flushFile()
sleep(milsecs = 2_000)
stderr.writeLine "some stderr3"
stderr.writeLine "some stderr4"
