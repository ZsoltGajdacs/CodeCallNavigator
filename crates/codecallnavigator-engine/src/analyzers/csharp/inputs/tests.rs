use super::*;

fn comment(body: &str) -> String {
    format!(
        "<!--\n{}\n{body}\n{}\n-->",
        "=".repeat(140),
        "=".repeat(140)
    )
}

fn root(path: &Path) -> String {
    comment(&escaped(&path.canonicalize().unwrap()))
}

fn begin(path: &Path) -> String {
    comment(&format!(
        "<Import Project=\"source.props\">\n\n{}",
        escaped(&path.canonicalize().unwrap())
    ))
}

fn end(path: &Path) -> String {
    comment(&format!(
        "</Import>\n\n{}",
        escaped(&path.canonicalize().unwrap())
    ))
}

#[test]
fn nested_frames_preserve_special_paths_and_ignore_unframed_comments() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("Root--&.csproj");
    let first = directory.path().join("Outer--&.props");
    let second = directory.path().join("Inner.props");
    for path in [&project, &first, &second] {
        fs::write(path, "<Project />").unwrap();
    }
    let text = format!(
        "{}<Project><!-- /not/an/input.props --><!-- <Import Project=\"/absent.props\" Condition=\"false\" /> -->{}{}{}{} </Project>",
        root(&project),
        begin(&first),
        begin(&second),
        end(&first),
        end(&project)
    );
    let actual = inventory(&text, &project).unwrap();
    let expected = [&project, &first, &second]
        .into_iter()
        .map(|path| path.canonicalize().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual.into_iter().collect::<BTreeSet<_>>(), expected);
}

#[test]
fn root_and_balanced_parent_frames_are_required() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("Root.csproj");
    let imported = directory.path().join("Input.props");
    fs::write(&project, "<Project />").unwrap();
    fs::write(&imported, "<Project />").unwrap();
    assert!(inventory("<Project/>", &project).is_err());
    let unclosed = format!("{}<Project>{}</Project>", root(&project), begin(&imported));
    assert!(inventory(&unclosed, &project).is_err());
    let wrong_parent = format!(
        "{}<Project>{}{}</Project>",
        root(&project),
        begin(&imported),
        end(&imported)
    );
    assert!(inventory(&wrong_parent, &project).is_err());
}

#[test]
fn escaped_filename_collisions_are_rejected_instead_of_guessed() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("a--b.props");
    let second = directory.path().join("a__b.props");
    fs::write(&first, "<Project />").unwrap();
    fs::write(&second, "<Project />").unwrap();
    assert!(resolve_path(&escaped(&first)).is_err());
}

#[test]
fn sdk_parentless_end_frames_still_require_balanced_imports() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("Root.csproj");
    let imported = directory.path().join("Input.props");
    fs::write(&project, "<Project />").unwrap();
    fs::write(&imported, "<Project />").unwrap();
    let text = format!(
        "{}<Project>{}{}</Project>",
        root(&project),
        begin(&imported),
        comment("</Import>")
    );
    assert_eq!(inventory(&text, &project).unwrap().len(), 2);
}

#[test]
#[ignore = "requires installed .NET 11 SDK"]
fn repeated_preprocessing_rejects_new_external_conditional_imports() {
    let directory = tempfile::tempdir().unwrap();
    let scope = directory.path().join("project");
    let job = directory.path().join("job");
    fs::create_dir(&scope).unwrap();
    fs::create_dir(&job).unwrap();
    let project = scope.join("Root.proj");
    let imported = directory.path().join("Optional.props");
    fs::write(&project, "<Project><Import Project=\"../Optional.props\" Condition=\"Exists('../Optional.props')\" /></Project>").unwrap();
    let project = project.canonicalize().unwrap();
    let dotnet = super::super::super::find_tool("dotnet").unwrap();
    let cancel = AtomicBool::new(false);
    let preprocessor = Preprocessor::new(&dotnet, &job, &cancel);
    let snapshot = preprocessor.capture(&project, 1).unwrap();
    fs::write(
        &imported,
        "<Project><PropertyGroup><Value>changed</Value></PropertyGroup></Project>",
    )
    .unwrap();
    let failure = preprocessor.verify(&snapshot).unwrap_err();
    assert!(format!("{failure:#}").contains("import inventory changed"));
}

#[test]
fn import_path_whitespace_is_preserved_instead_of_selecting_a_sibling() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("Root.csproj");
    let imported = directory.path().join("A.props ");
    let sibling = directory.path().join("A.props");
    for path in [&project, &imported, &sibling] {
        fs::write(path, "<Project />").unwrap();
    }
    let text = format!(
        "{}<Project>{}{}</Project>",
        root(&project),
        begin(&imported),
        end(&project)
    );
    let observed = inventory(&text, &project).unwrap();
    assert!(observed.contains(&imported.canonicalize().unwrap()));
    assert!(!observed.contains(&sibling.canonicalize().unwrap()));
}

#[test]
#[ignore = "requires installed .NET 11 SDK"]
fn context_import_swaps_are_rejected_even_when_the_union_is_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let scope = directory.path().join("project");
    let job = directory.path().join("job");
    fs::create_dir(&scope).unwrap();
    fs::create_dir(&job).unwrap();
    for name in ["A.props", "B.props"] {
        fs::write(directory.path().join(name), "<Project />").unwrap();
    }
    let project = scope.join("Root.proj");
    fs::write(&project, "<Project><Import Project=\"../A.props\" Condition=\"('$(DesignTimeBuild)' == 'True' And Exists('../marker')) Or ('$(DesignTimeBuild)' != 'True' And !Exists('../marker'))\"/><Import Project=\"../B.props\" Condition=\"('$(DesignTimeBuild)' == 'True' And !Exists('../marker')) Or ('$(DesignTimeBuild)' != 'True' And Exists('../marker'))\"/></Project>").unwrap();
    let project = project.canonicalize().unwrap();
    let dotnet = super::super::super::find_tool("dotnet").unwrap();
    let cancel = AtomicBool::new(false);
    let evaluator = Preprocessor::new(&dotnet, &job, &cancel);
    let snapshot = evaluator.capture(&project, 1).unwrap();
    fs::write(directory.path().join("marker"), "").unwrap();
    let failure = evaluator.verify(&snapshot).unwrap_err();
    assert!(format!("{failure:#}").contains("import inventory changed"));
}

#[test]
#[ignore = "requires installed .NET 11 SDK"]
fn solution_directory_is_a_literal_msbuild_property_without_global_flag_injection() {
    let directory = tempfile::tempdir().unwrap();
    let scope = directory.path().join("project");
    let job = directory.path().join("job");
    fs::create_dir(&scope).unwrap();
    fs::create_dir(&job).unwrap();
    let project = scope.join("Root.proj");
    fs::write(&project, "<Project />").unwrap();
    let dotnet = super::super::super::find_tool("dotnet").unwrap();
    let cancel = AtomicBool::new(false);
    let mut evaluator = Preprocessor::new(&dotnet, &job, &cancel);
    for (index, name) in [
        "solution;DesignTimeBuild=FALSE",
        "solution;DesignTimeBuild=FALSE,part%3Bliteral",
    ]
    .into_iter()
    .enumerate()
    {
        let solution = directory.path().join(name);
        fs::create_dir(&solution).unwrap();
        evaluator.solution_dir = Some(solution.clone());
        let mut command = evaluator.command(&project, &EvaluationContext::roots()[1]);
        command.arg("-getProperty:SolutionDir,DesignTimeBuild");
        let metadata = evaluator
            .query(command, &format!("literal-properties-{index}"))
            .unwrap();
        assert_eq!(
            metadata["Properties"]["SolutionDir"],
            format!("{}/", solution.display())
        );
        assert_eq!(metadata["Properties"]["DesignTimeBuild"], "True");
    }
}
