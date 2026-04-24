--lets pretend it exists
local ring_buffer = require("ring_buffer")
local buffer = ring_buffer.new({
	capacity = 2048,
})

--also lets pretend there's a metatable here
local Vec2 = {
	new = function(x, y)
		local vec = buffer:fetch()
		vec.x = x
		vec.y = y
		return vec
	end,

	--making sure the vec will persist
	box = function(vec)
		return { x = vec.x, y = vec.y }
	end,

	__add = function(a, b)
		local result = buffer:fetch()
		return result
	end,
	--other metafuntions...
}

--a temp vec for calculations within a frame
local vec = Vec2.new(1, 2) + Vec2.new(30, 40)


-- ...

player.positon = Vec2.box(vec)

--You'd also need to have copy semantics form rust to ensure that this doesn't happen:
local v1 = Vec2.new(1, 2)
local v1_ref = v1
v1_ref.x = 0 --this shouldn't affect v1
