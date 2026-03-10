--[[
fn main() {
    let vec: (float, float) = (1,2);
    vec = vec.normalize().rotate((0,0), 1) * 2;
}
--]]

---@class lopa.Pool<T: table>
---  @field freed table<T, nil>
---  @field fetched table<T, nil>
local pool = {}
pool.__index = pool

---@package
---@return T
function pool:fetch_new()
	local value = {}
	self.fetched[value] = true
	return value
end

---@return T
function pool:fetch()
	if next(self.freed) ~= nil then
		local free_value = next(self.freed) --[[@as T]]
		self.fetched[free_value] = true
		return free_value
	end

	return self:fetch_new()
end

---@param value T
function pool:free(value)
	local fetched = self.fetched[value]
	self.fetched[value] = nil
	if not fetched then
		return nil
	end

	self.freed[value] = true
end

function pool:free_all()
	for v, _ in pairs(self.fetched) do
		self.freed[v] = true
	end
	self.fetched = {}
end

---@class lopa.Pool.defs
Pool = {
	---@generic T
	---@param _type? `T`
	---@return lopa.Pool<T>
	new = function(_type)
		return setmetatable({
			freed = {},
			fetched = {},
		}, pool)
	end,
}

local tuple_pool = Pool.new()

function t2(i1, i2)
	local t = tuple_pool:fetch()
	t[1] = i1
	t[2] = i2
	return t
end

function add_t2(t1, t2)
	local t = tuple_pool:fetch()
	t[1] = t1[1] + t2[1]
	t[2] = t2[2] + t2[2]
	return t
end

function clear_all()
	tuple_pool:free_all()
end

for i = 1, 20 do
	local v = t2(1, 2)
	local v1 = add_t2(v, t2(2, 3))
	clear_all()
end

function tablelength(T)
	local count = 0
	for _ in pairs(T) do
		count = count + 1
	end
	return count
end

---@type metatable
local m = {
	__sub = function()
		print("adding number!")
	end,
}
debug.setmetatable(0, m)

collectgarbage('setpause')
