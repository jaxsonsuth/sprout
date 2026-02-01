local M = {}

local config = require("sprout.config")
local state = require("sprout.state")
local server = require("sprout.server")
local client = require("sprout.client")
local display = require("sprout.display")

function M.setup(opts)
	config.setup(opts)

	-- Set keymaps (insert mode)
	vim.keymap.set("i", config.options.keymaps.trigger, M.trigger, { desc = "Sprout: trigger completion" })
	vim.keymap.set("i", config.options.keymaps.accept, M.accept, { desc = "Sprout: accept completion" })
	vim.keymap.set("i", config.options.keymaps.cancel, M.cancel, { desc = "Sprout: cancel completion" })
end

function M.trigger()
	if state.current.active then
		return
	end -- Already running

	local cursor_pos = vim.api.nvim_win_get_cursor(0)
	local line = vim.api.nvim_get_current_line()
	local prompt = line:sub(1, cursor_pos[2])

	state.current.cursor_pos = cursor_pos
	state.current.text = ""
	state.current.active = true

	server.ensure_running(function()
		local filepath = vim.api.nvim_buf_get_name(0)
		local session_id = state.sessions[filepath]

		local function start_stream(sid)
			state.sessions[filepath] = sid
			state.current.job_id = client.stream_completion(sid, prompt, function(token) -- on_token
				state.current.text = state.current.text .. token
				display.show(state.current.text, state.current.cursor_pos)
			end, function() -- on_done
				state.current.active = false
				state.current.job_id = nil
			end)
		end

		if session_id then
			start_stream(session_id)
		else
			client.create_session(start_stream)
		end
	end)
end

function M.accept()
	if state.current.text == "" then
		return
	end

	-- Insert the text at cursor position
	local pos = state.current.cursor_pos
	local line = vim.api.nvim_get_current_line()
	local before = line:sub(1, pos[2])
	local after = line:sub(pos[2] + 1)

	local new_text = before .. state.current.text .. after
	local new_lines = vim.split(new_text, "\n", { plain = true })

	-- Replace current line with new lines
	vim.api.nvim_buf_set_lines(0, pos[1] - 1, pos[1], false, new_lines)

	-- Move cursor to end of inserted text
	local last_line = pos[1] - 1 + #new_lines
	local last_col = #new_lines[#new_lines] - #after
	vim.api.nvim_win_set_cursor(0, { last_line, last_col })

	-- Clear display and reset state
	display.clear()
	state.reset()
end

function M.cancel()
	-- Stop the streaming job if running
	if state.current.job_id then
		vim.fn.jobstop(state.current.job_id)
	end

	-- Clear display and reset state
	display.clear()
	state.reset()
end

return M
