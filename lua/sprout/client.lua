local M = {}
local config = require("sprout.config")

function M.create_session(callback)
	local url = config.options.server_url .. "/create_session"

	vim.fn.jobstart({ "curl", "-s", url }, {
		stdout_buffered = true,
		on_stdout = function(_, data)
			local json_str = table.concat(data, "")
			local ok, parsed = pcall(vim.json.decode, json_str)
			if ok and parsed.session_id then
				callback(parsed.session_id)
			else
				vim.notify("Sprout: failed to create session", vim.log.levels.ERROR)
			end
		end,
	})
end

function M.stream_completion(session_id, prompt, on_token, on_done)
	local url = config.options.server_url .. "/compleat/stream/" .. session_id
	local body = vim.json.encode({ text = prompt })

	local job_id = vim.fn.jobstart({
		"curl",
		"-N",
		"-s",
		"-X",
		"POST",
		"-H",
		"Content-Type: application/json",
		"-d",
		body,
		url,
	}, {
		on_stdout = function(_, data)
			for _, line in ipairs(data) do
				-- Parse SSE format: "data: <token>"
				local token = line:match("^data: (.*)$")
				if token then
					on_token(token)
				end
			end
		end,
		on_exit = function()
			on_done()
		end,
	})

	return job_id
end

return M
