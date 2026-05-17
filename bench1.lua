function wrap(f)
	f()
end

local t = {}
for i = 1, 1e6 do
	wrap(function()
		t[i] = i
	end)
end
