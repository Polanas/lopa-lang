
local ffi = require("ffi")
ffi.cdef([[
  typedef struct {double x,y; } vec2;
]])
local vec2s = ffi.new("vec2[2048]")

local count = 0
function new_vec2(x, y)
	local v = vec2s[count]
	count = count + 1
	v.x = x
	v.y = y
	return v
end

ffi.metatype("vec2", {
	__add = function(a, b)
		return new_vec2(a.x + b.x, a.y + b.y)
	end,
})

local first = new_vec2(1, 2)
local second = new_vec2(3, 4)
local sum = first + second
print(sum.x, sum.y)
