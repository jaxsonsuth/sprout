local M = {}
local config = require("sprout.config")
M.ns = vim.api.nvim_create_namespace("sprout")
function M.show(text, cursor_pos)
	M.clear() -- Clear any existing ghost text

	if text == "" then
		return
	end

	local buf = vim.api.nvim_get_current_buf()
	local line = cursor_pos[1] - 1 -- 0-indexed
	local col = cursor_pos[2]

	-- Split text into lines
	local lines = vim.split(text, "\n", { plain = true })

	-- First line: inline overlay at cursor
	local first_line = lines[1] or ""
	local virt_lines = {}

	-- Build virt_lines for lines 2+
	for i = 2, #lines do
		table.insert(virt_lines, { { lines[i], config.options.highlight } })
	end

	-- Set the extmark
	vim.api.nvim_buf_set_extmark(buf, M.ns, line, col, {
		virt_text = { { first_line, config.options.highlight } },
		virt_text_pos = "overlay",
		virt_lines = #virt_lines > 0 and virt_lines or nil,
	})
end
function M.clear()
	local buf = vim.api.nvim_get_current_buf()
	vim.api.nvim_buf_clear_namespace(buf, M.ns, 0, -1)
end
return M
