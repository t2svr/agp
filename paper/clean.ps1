if ([io.directory]::Exists("./paper/builds/")) {
    $files = Get-ChildItem  -Path ./paper/builds/ -Recurse
    foreach ($f in $files) {
        Remove-Item -Path $f.FullName
    } 
} else {
    [io.directory]::CreateDirectory("./paper/builds/")
}
