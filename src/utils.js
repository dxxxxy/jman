export const getVersionFromFolderName = (jdk) => {
    const results = /jdk(|-)([0-9._]+)/.exec(jdk);
    return results[2];
}