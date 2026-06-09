local ffi = require("ffi")

ffi.cdef([[
  typedef struct {
    float x;
    float y;
  } Vec2;
]])

ffi.metatype(ffi.typeof("Vec2"), {
	__add = function(a, b)
		return ffi.new("Vec2", a.x + a.x, b.y + b.y)
	end,
})

local array = ffi.new("Vec2[?]", 10)
local first = array[-1]

local first = ffi.cast("Vec2*", first)

print(ffi.typeof(first))

local sum = first + first

print(sum.x, sum.y)
