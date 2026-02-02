local M = {}

M.job_id = nil

function M.get_plugin_dir()
	-- Get the path to this Lua file, then navigate to plugin root
	local source = debug.getinfo(1, "S").source:sub(2) -- Remove leading @
	return vim.fn.fnamemodify(source, ":h:h:h") -- lua/sprout/server.lua -> repo root
end

function M.get_binary_path()
	return M.get_plugin_dir() .. "/server/target/release/sprout-server"
end

function M.start()
	if M.job_id then
		return
	end -- Already running

	local binary = M.get_binary_path()
	if vim.fn.filereadable(binary) == 0 then
		vim.notify("Sprout: server binary not found. Run 'cargo build --release' in server/", vim.log.levels.ERROR)
		return
	end

	local plugin_dir = M.get_plugin_dir()
	M.job_id = vim.fn.jobstart({ binary }, {
		cwd = plugin_dir .. "/server", -- Run from server/ so ../models/ resolves correctly
		detach = true,
		on_exit = function()
			M.job_id = nil
		end,
	})
end

function M.stop()
	if M.job_id then
		vim.fn.jobstop(M.job_id)
		M.job_id = nil
	end
end

function M.ensure_running(callback)
	-- Try health endpoint
	vim.fn.jobstart({ "curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", "http://127.0.0.1:8000/health" }, {
		stdout_buffered = true,
		on_stdout = function(_, data)
			local code = table.concat(data, "")
			if code == "200" then
				callback(true)
			else
				M.start()
				-- Give server a moment to start, then callback
				vim.defer_fn(function()
					callback(true)
				end, 1000)
			end
		end,
	})
end

return M
