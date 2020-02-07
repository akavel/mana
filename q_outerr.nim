import os

stdout.writeLine "some stdout"
stderr.writeLine "some stderr"
stderr.writeLine "some stderr2"
sleep(milsecs = 5_000)
stderr.writeLine "some stderr3"
stderr.writeLine "some stderr4"
