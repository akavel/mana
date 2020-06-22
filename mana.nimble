# Package

version       = "0.1.0"
author        = "Mateusz Czapliński"
description   = "Personal configuration and provisioning"
license       = "AGPL-3.0"
srcDir        = "src"
bin           = @["mana"]



# Dependencies

requires "nim >= 1.2.0"
requires "npeg 0.22.2"
# result 0.1 is broken at time of writing
requires "result#bdc585bf9f3ad0acaad18c7d12deab172373b5f4"
