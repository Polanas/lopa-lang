-- ---@generic T, R, R1
-- ---@param f sync fun(...: T...): R1, R...
-- ---@param ... T...
-- ---@return boolean, R1|string, R...
-- function pcall(f, ...) end
--
-- --  fn pcall<...T, ...R, F>(f: F, ...T) -> ...R {} where F: fn(...T) -> ...R
-- --
-- --  fn test(a: "hi" | "hey" | "hello")
--
-- --global b = a
-- --global a = b
--

call_once = function(f, arg)
	do
		f(arg)
	end
end
main = function()
	do
		call_once(function(value)
			do
				stack_0 = print(value)
			end
			return
		end, 20)
	end
end
pcall()
