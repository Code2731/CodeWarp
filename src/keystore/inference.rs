keystore_entry_std!(
    inference_cmd_entry,
    read_inference_command,
    write_inference_command,
    clear_inference_command,
    "inference_start_command"
);

keystore_entry_std!(
    inference_bin_entry,
    read_inference_binary,
    write_inference_binary,
    clear_inference_binary,
    "inference_binary_path"
);

keystore_entry_no_clear!(
    model_dir_entry,
    read_model_dir,
    write_model_dir,
    "model_download_dir"
);
