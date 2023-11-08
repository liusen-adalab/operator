pub extern crate async_process;
pub extern crate tracing;

#[macro_export]
macro_rules! async_cmd {
    ($cmd:literal, $($tts:tt)*) => {{
        $crate::async_cmd!(@run $cmd, $($tts)*)
    }};

    ($cmd:literal $(,)?) => {{
        $crate::async_cmd!(@run $cmd,)
    }};

    (@run $cmd:literal, $($tts:tt)*) => {{
        use std::ffi::OsStr;
        use $crate::macros::async_cmd::async_process::{Command, Stdio};
        use anyhow::{anyhow, Context};
        use $crate::macros::async_cmd::tracing;

        let mut args: Vec<&(dyn AsRef<OsStr> + Send + Sync)> = vec![];
        $crate::async_cmd!(@collect args, $($tts)*);

        let mut cmd = Command::new($cmd);
        cmd.args(&args);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);


        let child = cmd.spawn().context("error: cmd spawn")?;
        let output = child.output().await.context("error: wait for cmd output")?;

        let mut cmd = $cmd.to_string();
        for arg in args {
            cmd += &format!(r#" "{}""#, arg.as_ref().to_string_lossy());
        }

        if !output.status.success() {
            tracing::error!(?output, %cmd, concat!("cmd failed"));
            return Err(anyhow!("[{}] failed. output = {:?}. cmd => {} <=", $cmd, output, cmd));
        } else {
            tracing::debug!(%cmd, concat!("cmd success"));
        }
        output.stdout
    }};

    (@collect $args:expr $(,)?) => {};

    (@collect $args:expr, [$dyn_args:expr], $($tts:tt)+) => {
        for arg in &*$dyn_args {
            $args.push(arg);
        }
        $crate::async_cmd!(@collect $args, $($tts)*);
    };

    (@collect $args:expr, [$dyn_args:expr] $(,)?) => {
        for arg in &$dyn_args {
            $args.push(arg);
        }
    };

    (@collect $args:expr, $arg:expr, $($tts:tt)+) => {
        let arg: &(dyn AsRef<OsStr> + Sync + Send) = &$arg;
        $args.push(arg);
        $crate::async_cmd!(@collect $args, $($tts)*)
    };

    (@collect $args:expr, $arg:expr $(,)?) => {
        $args.push(&$arg);
    };

}

#[cfg(test)]
mod test {

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn cmd_test() -> anyhow::Result<()> {
        async_cmd!("ls", "-l", "-a");

        Ok(())
    }
}
