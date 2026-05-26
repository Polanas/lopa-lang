local ffi = require("ffi")

local array = ffi.new("uint8_t[?]", 10)
array[0] = 51
array[1] = 32
array[2] = 51

local str = ffi.string(array, 10)
print(str)
