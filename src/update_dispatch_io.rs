impl App {
    pub(crate) fn dispatch_io(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::FileDropped(path) => Some(self.on_file_dropped(path.clone())),
            Message::FileDragHover => Some(Self::file_drag_hover()),
            Message::FileReadDone(path, content) => {
                Some(self.on_file_read_done(path.clone(), content.clone()))
            }
            Message::FileAttachError(msg) => Some(self.file_attach_error(msg.clone())),
            Message::McpNameChanged(v) => Some(self.update_mcp_name_input(v.clone())),
            Message::McpCommandChanged(v) => Some(self.update_mcp_command_input(v.clone())),
            Message::AddMcpServer => Some(self.add_mcp_server()),
            Message::RemoveMcpServer(idx) => Some(self.remove_mcp_server(*idx)),
            Message::McpToolsLoaded(server_name, tools) => {
                Some(self.on_mcp_tools_loaded(server_name, tools.clone()))
            }
            Message::McpToolsFailed(msg) => Some(self.on_mcp_tools_failed(msg)),
            Message::McpToolResult(tool_call_id, result) => {
                Some(self.on_mcp_tool_result(tool_call_id, result.clone()))
            }
            Message::PtyToggle => Some(self.toggle_pty()),
            Message::PtyStart => Some(self.pty_start()),
            Message::PtyLine(line) => Some(self.on_pty_line(line)),
            Message::PtyExited => Some(self.on_pty_exited()),
            Message::PtyInputChanged(v) => Some(self.set_pty_input(v.clone())),
            Message::PtySend => Some(self.send_pty_input()),
            Message::PtyCtrlC => Some(self.pty_ctrl_c()),
            Message::PtyClear => Some(self.pty_clear()),
            Message::RemoveAttachment(idx) => Some(self.remove_attachment(*idx)),
            Message::ClearAttachments => Some(self.clear_attachments()),
            _ => None,
        }
    }
}
