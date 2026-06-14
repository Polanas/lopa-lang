local ffi = require("ffi")

ffi.cdef([[
    typedef struct {
      float x;
      float y;
    } Vec2;
]])

local vec2s = ffi.new("Vec2[?]", 1e6)
vec2s[0].x = 1
vec2s[0].y = 2

for i = 1, 1e6 - 1 do
	vec2s[i].x = vec2s[i - 1].x + 2
end

print("done2")
