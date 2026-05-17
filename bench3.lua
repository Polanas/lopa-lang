function wrap(f)
	f[1](f)
end
local t = {}
local function lambda(U)
	local a = 2 + U[3]
	U[2] = U[3] * a
end

for i = 1, 1e6 do
	wrap({ lambda, t, i })
end
