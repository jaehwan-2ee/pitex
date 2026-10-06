//! Native font tags, extended units and diagnostic stream routing against pdfTeX.
use pitex_native_tools::{preview_regression::{self,output_timeout,Session},TempDir};
use serde_json::json;
use std::{collections::BTreeMap,error::Error,fs,process::{Command,Stdio},time::Duration};
const TIMEOUT:Duration=Duration::from_secs(60);
fn records(log:&str)->BTreeMap<String,String>{log.lines().filter(|v|v.starts_with("CORE-AUDIT-")).filter_map(|v|v.split_once('=').map(|(k,v)|(k.into(),v.into()))).collect()}
fn run()->Result<(),Box<dyn Error>>{
 preview_regression::watch_interrupt();let bin=fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
 let temp=TempDir::new("pitex-core-controls-")?;let work=fs::canonicalize(temp.path())?;
 let cases=[
  ("font-tags",r"\font\f=cmr10\font\ext=cmex10
\typeout{CORE-AUDIT-TAG-A=\the\tagcode\f`A}
\typeout{CORE-AUDIT-TAG-LIST=\the\tagcode\ext0}
\setbox0=\hbox{\f office AV}\typeout{CORE-AUDIT-WIDTH-BEFORE=\the\wd0}
\tagcode\f`A=-4\typeout{CORE-AUDIT-KEEP=\the\tagcode\f`A}
{\tagcode\f`f=-1}\typeout{CORE-AUDIT-GLOBAL=\the\tagcode\f`f}
\setbox0=\hbox{\f office AV}\typeout{CORE-AUDIT-WIDTH-AFTER=\the\wd0}
\tagcode\ext0=-2\typeout{CORE-AUDIT-LIST-CLEAR=\the\tagcode\ext0}
\tagcode\f`A=3\typeout{CORE-AUDIT-POSITIVE=\the\tagcode\f`A}
\tagcode\f`A=-7\typeout{CORE-AUDIT-CLEAR=\the\tagcode\f`A}
\f office AV."),
  ("extended-units",r"\dimen0=1nd\typeout{CORE-AUDIT-ND=\the\dimen0}\dimen0=12.75nd\typeout{CORE-AUDIT-ND-FRACTION=\the\dimen0}\dimen0=1nc\typeout{CORE-AUDIT-NC=\the\dimen0}\dimen0=-2.95nc\typeout{CORE-AUDIT-NC-FRACTION=\the\dimen0}\dimen0=12nd\dimen1=1nc\typeout{CORE-AUDIT-EQUIVALENT=\the\dimen0,\the\dimen1}Extended units."),
  ("show-stream",r"\newwrite\traceout\newread\tracein
\immediate\openout\traceout=trace-output.txt
\showstream=\traceout\def\TraceMacro{TraceRecoveredMarker}\show\TraceMacro
\dimen0=12pt\showthe\dimen0\setbox0=\hbox{\vrule height5pt width3pt}\showbox0\showlists
\showstream=-1\immediate\closeout\traceout
\openin\tracein=trace-output.txt
\readline\tracein to\firstline\readline\tracein to\secondline\readline\tracein to\thirdline\readline\tracein to\fourthline
\closein\tracein\typeout{CORE-AUDIT-RECOVERED=\pdfmatch{TraceRecoveredMarker}{\firstline\secondline\thirdline\fourthline}}
\typeout{CORE-AUDIT-STREAM=\the\showstream}Diagnostic stream."),
 ];
 for (name,body) in cases {
  let root=work.join(name);fs::create_dir(&root)?;let main=root.join("main.tex");
  let source=format!("\\documentclass{{article}}\n\\begin{{document}}\nControlMarkerOriginal.\n\n{body}\n\\end{{document}}\n");fs::write(&main,&source)?;
  let reference=root.join("reference");fs::create_dir(&reference)?;
  let output=output_timeout(Command::new("pdflatex").args(["-interaction=nonstopmode","-halt-on-error","-no-shell-escape"]).arg(format!("-output-directory={}",reference.display())).arg("main.tex").current_dir(&root),TIMEOUT)?;
  if !output.status.success(){return Err(format!("{name} reference: {}",String::from_utf8_lossy(&output.stdout)).into());}
  let expected=records(&fs::read_to_string(reference.join("main.log"))?);
  let out=root.join("preview");fs::create_dir(&out)?;
  let mut session=Session::spawn(Command::new(bin.join("pitex-preview")).arg("--root").arg(&root).args(["--main","main.tex","--out"]).arg(&out).arg("--cache").arg(work.join("cache")).stderr(Stdio::null()),&out)?;
  for generation in [1u64,2]{session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source.replace("ControlMarkerOriginal",&format!("ControlMarker{generation}"))}],"closed":[]}))?;
   let event=session.wait_published(generation,TIMEOUT)?;if event["errors"]!=0{return Err(format!("{name}: {event}").into());}
   let log=fs::read_to_string(event["log"].as_str().ok_or("log missing")?)?;
   if records(&log)!=expected{return Err(format!("{name}: expected {expected:?}, actual {:?}",records(&log)).into());}
   if name=="show-stream"&&(expected.get("CORE-AUDIT-RECOVERED").map(String::as_str)!=Some("1")||log.lines().any(|s|s.starts_with("> \\TraceMacro="))){return Err("show diagnostics were lost or leaked into normal log".into());}
   if fs::read_to_string(&main)?!=source{return Err("preview changed source".into());}
  }preview_regression::pass(name,Some("pdftex"),&[&source],"exact reference controls and two unsaved generations")?;
 }Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("{error}");std::process::exit(if preview_regression::interrupted(error.as_ref()){130}else{1});}}
