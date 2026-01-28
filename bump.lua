local ffi = require("ffi")
ffi.cdef([[
  typedef struct {
    int32_t $;
    int32_t $;
  } Point;

  typedef struct {
    Point p1;
  } Points;
]], "x", "x")

local points = ffi.new("Points", {
	p1 = {
		x = 1,
		y = 2,
	},
})
print(points.p1.x, points.p1.y)
