local M = {}

M.sessions = {} -- { [filepath] = "uuid-..." }
M.current = {
	active = false, -- Is a completion in progress?
	job_id = nil, -- curl job ID (to cancel)
	text = "", -- Accumulated completion text
	cursor_pos = nil, -- Where completion started {line, col}
}
function M.reset() -- ADD: reset current completion
	M.current = { active = false, job_id = nil, text = "", cursor_pos = nil }
end
return M
