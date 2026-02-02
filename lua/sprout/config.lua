local M = {}

M.defaults = {
	server_url = "http://127.0.0.1:8000",
	keymaps = {
		trigger = "<C-a>",
		accept = "<C-s>",
		cancel = "<C-j>",
	},
	highlight = "Comment",
	auto_start_server = true,
}

M.options = {}

function M.setup(opts) -- ADD: merge function
	M.options = vim.tbl_deep_extend("force", M.defaults, opts or {})
end

return M
