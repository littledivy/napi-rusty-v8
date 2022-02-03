// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

// @ts-check
/// <reference path="./compiler.d.ts" />
// deno-lint-ignore-file no-undef

// This module is the entry point for "compiler" isolate, ie. the one
// that is created when Deno needs to type check TypeScript, and in some
// instances convert TypeScript to JavaScript.

// Removes the `__proto__` for security reasons.  This intentionally makes
// Deno non compliant with ECMA-262 Annex B.2.2.1
delete Object.prototype.__proto__;

((window) => {
  /** @type {DenoCore} */
  const core = window.Deno.core;

  let logDebug = false;
  let logSource = "JS";

  /** @type {string=} */
  let cwd;

  // The map from the normalized specifier to the original.
  // TypeScript normalizes the specifier in its internal processing,
  // but the original specifier is needed when looking up the source from the runtime.
  // This map stores that relationship, and the original can be restored by the
  // normalized specifier.
  // See: https://github.com/denoland/deno/issues/9277#issuecomment-769653834
  const normalizedToOriginalMap = new Map();

  function setLogDebug(debug, source) {
    logDebug = debug;
    if (source) {
      logSource = source;
    }
  }

  function printStderr(msg) {
    core.print(msg, true);
  }

  function debug(...args) {
    if (logDebug) {
      const stringifiedArgs = args.map((arg) =>
        typeof arg === "string" ? arg : JSON.stringify(arg)
      ).join(" ");
      printStderr(`DEBUG ${logSource} - ${stringifiedArgs}\n`);
    }
  }

  function error(...args) {
    const stringifiedArgs = args.map((arg) =>
      typeof arg === "string" || arg instanceof Error
        ? String(arg)
        : JSON.stringify(arg)
    ).join(" ");
    printStderr(`ERROR ${logSource} = ${stringifiedArgs}\n`);
  }

  class AssertionError extends Error {
    constructor(msg) {
      super(msg);
      this.name = "AssertionError";
    }
  }

  function assert(cond, msg = "Assertion failed.") {
    if (!cond) {
      throw new AssertionError(msg);
    }
  }

  /** @type {Map<string, ts.SourceFile>} */
  const sourceFileCache = new Map();

  /** @type {string[]=} */
  let scriptFileNamesCache;

  /** @type {Map<string, string>} */
  const scriptVersionCache = new Map();

  /** @param {ts.DiagnosticRelatedInformation} diagnostic */
  function fromRelatedInformation({
    start,
    length,
    file,
    messageText: msgText,
    ...ri
  }) {
    let messageText;
    let messageChain;
    if (typeof msgText === "object") {
      messageChain = msgText;
    } else {
      messageText = msgText;
    }
    if (start !== undefined && length !== undefined && file) {
      const startPos = file.getLineAndCharacterOfPosition(start);
      const sourceLine = file.getFullText().split("\n")[startPos.line];
      const fileName = file.fileName;
      return {
        start: startPos,
        end: file.getLineAndCharacterOfPosition(start + length),
        fileName,
        messageChain,
        messageText,
        sourceLine,
        ...ri,
      };
    } else {
      return {
        messageChain,
        messageText,
        ...ri,
      };
    }
  }

  /** @param {ts.Diagnostic[]} diagnostics */
  function fromTypeScriptDiagnostic(diagnostics) {
    return diagnostics.map(({ relatedInformation: ri, source, ...diag }) => {
      /** @type {any} */
      const value = fromRelatedInformation(diag);
      value.relatedInformation = ri
        ? ri.map(fromRelatedInformation)
        : undefined;
      value.source = source;
      return value;
    });
  }

  // Using incremental compile APIs requires that all
  // paths must be either relative or absolute. Since
  // analysis in Rust operates on fully resolved URLs,
  // it makes sense to use the same scheme here.
  const ASSETS = "asset:///";

  /** Diagnostics that are intentionally ignored when compiling TypeScript in
   * Deno, as they provide misleading or incorrect information. */
  const IGNORED_DIAGNOSTICS = [
    // TS1208: All files must be modules when the '--isolatedModules' flag is
    // provided.  We can ignore because we guarantee that all files are
    // modules.
    1208,
    // TS1375: 'await' expressions are only allowed at the top level of a file
    // when that file is a module, but this file has no imports or exports.
    // Consider adding an empty 'export {}' to make this file a module.
    1375,
    // TS2306: File 'file:///Users/rld/src/deno/cli/tests/testdata/subdir/amd_like.js' is
    // not a module.
    2306,
    // TS2688: Cannot find type definition file for '...'.
    // We ignore because type defintion files can end with '.ts'.
    2688,
    // TS2691: An import path cannot end with a '.ts' extension. Consider
    // importing 'bad-module' instead.
    2691,
    // TS2792: Cannot find module. Did you mean to set the 'moduleResolution'
    // option to 'node', or to add aliases to the 'paths' option?
    2792,
    // TS5009: Cannot find the common subdirectory path for the input files.
    5009,
    // TS5055: Cannot write file
    // 'http://localhost:4545/subdir/mt_application_x_javascript.j4.js'
    // because it would overwrite input file.
    5055,
    // TypeScript is overly opinionated that only CommonJS modules kinds can
    // support JSON imports.  Allegedly this was fixed in
    // Microsoft/TypeScript#26825 but that doesn't seem to be working here,
    // so we will ignore complaints about this compiler setting.
    5070,
    // TS7016: Could not find a declaration file for module '...'. '...'
    // implicitly has an 'any' type.  This is due to `allowJs` being off by
    // default but importing of a JavaScript module.
    7016,
  ];

  const SNAPSHOT_COMPILE_OPTIONS = {
    esModuleInterop: true,
    jsx: ts.JsxEmit.React,
    module: ts.ModuleKind.ESNext,
    noEmit: true,
    strict: true,
    target: ts.ScriptTarget.ESNext,
  };

  class ScriptSnapshot {
    /** @type {string} */
    specifier;
    /** @type {string} */
    version;
    /**
     * @param {string} specifier
     * @param {string} version
     */
    constructor(specifier, version) {
      this.specifier = specifier;
      this.version = version;
    }
    /**
     * @param {number} start
     * @param {number} end
     * @returns {string}
     */
    getText(start, end) {
      const { specifier, version } = this;
      debug(
        `snapshot.getText(${start}, ${end}) specifier: ${specifier} version: ${version}`,
      );
      return core.opSync("op_get_text", { specifier, version, start, end });
    }
    /**
     * @returns {number}
     */
    getLength() {
      const { specifier, version } = this;
      debug(`snapshot.getLength() specifier: ${specifier} version: ${version}`);
      return core.opSync("op_get_length", { specifier, version });
    }
    /**
     * @param {ScriptSnapshot} oldSnapshot
     * @returns {ts.TextChangeRange | undefined}
     */
    getChangeRange(oldSnapshot) {
      const { specifier, version } = this;
      const { version: oldVersion } = oldSnapshot;
      const oldLength = oldSnapshot.getLength();
      debug(
        `snapshot.getLength() specifier: ${specifier} oldVersion: ${oldVersion} version: ${version}`,
      );
      return core.opSync(
        "op_get_change_range",
        { specifier, oldLength, oldVersion, version },
      );
    }
    dispose() {
      const { specifier, version } = this;
      debug(`snapshot.dispose() specifier: ${specifier} version: ${version}`);
      core.opSync("op_dispose", { specifier, version });
    }
  }

  /** Error thrown on cancellation. */
  class OperationCanceledError extends Error {
  }

  /**
   * Inspired by ThrottledCancellationToken in ts server.
   *
   * We don't want to continually call back into Rust and so
   * we throttle cancellation checks to only occur once
   * in a while.
   * @implements {ts.CancellationToken}
   */
  class ThrottledCancellationToken {
    #lastCheckTimeMs = 0;

    isCancellationRequested() {
      const timeMs = Date.now();
      // TypeScript uses 20ms
      if ((timeMs - this.#lastCheckTimeMs) < 20) {
        return false;
      }

      this.#lastCheckTimeMs = timeMs;
      return core.opSync("op_is_cancelled", {});
    }

    throwIfCancellationRequested() {
      if (this.isCancellationRequested()) {
        throw new OperationCanceledError();
      }
    }
  }

  /** @type {ts.CompilerOptions} */
  let compilationSettings = {};

  /** @type {ts.LanguageService} */
  let languageService;

  /** An object literal of the incremental compiler host, which provides the
   * specific "bindings" to the Deno environment that tsc needs to work.
   *
   * @type {ts.CompilerHost & ts.LanguageServiceHost} */
  const host = {
    fileExists(specifier) {
      debug(`host.fileExists("${specifier}")`);
      specifier = normalizedToOriginalMap.get(specifier) ?? specifier;
      return core.opSync("op_exists", { specifier });
    },
    readFile(specifier) {
      debug(`host.readFile("${specifier}")`);
      return core.opSync("op_load", { specifier }).data;
    },
    getCancellationToken() {
      // createLanguageService will call this immediately and cache it
      return new ThrottledCancellationToken();
    },
    getSourceFile(
      specifier,
      languageVersion,
      _onError,
      _shouldCreateNewSourceFile,
    ) {
      debug(
        `host.getSourceFile("${specifier}", ${
          ts.ScriptTarget[languageVersion]
        })`,
      );
      let sourceFile = sourceFileCache.get(specifier);
      if (sourceFile) {
        return sourceFile;
      }

      // Needs the original specifier
      specifier = normalizedToOriginalMap.get(specifier) ?? specifier;

      /** @type {{ data: string; hash?: string; scriptKind: ts.ScriptKind }} */
      const { data, hash, scriptKind } = core.opSync(
        "op_load",
        { specifier },
      );
      assert(
        data != null,
        `"data" is unexpectedly null for "${specifier}".`,
      );
      sourceFile = ts.createSourceFile(
        specifier,
        data,
        languageVersion,
        false,
        scriptKind,
      );
      sourceFile.moduleName = specifier;
      sourceFile.version = hash;
      sourceFileCache.set(specifier, sourceFile);
      return sourceFile;
    },
    getDefaultLibFileName() {
      return `${ASSETS}/lib.esnext.d.ts`;
    },
    getDefaultLibLocation() {
      return ASSETS;
    },
    writeFile(fileName, data, _writeByteOrderMark, _onError, sourceFiles) {
      debug(`host.writeFile("${fileName}")`);
      let maybeSpecifiers;
      if (sourceFiles) {
        maybeSpecifiers = sourceFiles.map((sf) => sf.moduleName);
      }
      return core.opSync(
        "op_emit",
        { maybeSpecifiers, fileName, data },
      );
    },
    getCurrentDirectory() {
      debug(`host.getCurrentDirectory()`);
      return cwd ?? core.opSync("op_cwd", null);
    },
    getCanonicalFileName(fileName) {
      return fileName;
    },
    useCaseSensitiveFileNames() {
      return true;
    },
    getNewLine() {
      return "\n";
    },
    resolveModuleNames(specifiers, base) {
      debug(`host.resolveModuleNames()`);
      debug(`  base: ${base}`);
      debug(`  specifiers: ${specifiers.join(", ")}`);
      /** @type {Array<[string, ts.Extension] | undefined>} */
      const resolved = core.opSync("op_resolve", {
        specifiers,
        base,
      });
      if (resolved) {
        const result = resolved.map((item) => {
          if (item) {
            const [resolvedFileName, extension] = item;
            return {
              resolvedFileName,
              extension,
              isExternalLibraryImport: false,
            };
          }
          return undefined;
        });
        result.length = specifiers.length;
        return result;
      } else {
        return new Array(specifiers.length);
      }
    },
    createHash(data) {
      return core.opSync("op_create_hash", { data }).hash;
    },

    // LanguageServiceHost
    getCompilationSettings() {
      debug("host.getCompilationSettings()");
      return compilationSettings;
    },
    getScriptFileNames() {
      debug("host.getScriptFileNames()");
      // tsc requests the script file names multiple times even though it can't
      // possibly have changed, so we will memoize it on a per request basis.
      if (scriptFileNamesCache) {
        return scriptFileNamesCache;
      }
      return scriptFileNamesCache = core.opSync("op_script_names", undefined);
    },
    getScriptVersion(specifier) {
      debug(`host.getScriptVersion("${specifier}")`);
      const sourceFile = sourceFileCache.get(specifier);
      if (sourceFile) {
        return sourceFile.version ?? "1";
      }
      // tsc requests the script version multiple times even though it can't
      // possibly have changed, so we will memoize it on a per request basis.
      if (scriptVersionCache.has(specifier)) {
        return scriptVersionCache.get(specifier);
      }
      const scriptVersion = core.opSync("op_script_version", { specifier });
      scriptVersionCache.set(specifier, scriptVersion);
      return scriptVersion;
    },
    getScriptSnapshot(specifier) {
      debug(`host.getScriptSnapshot("${specifier}")`);
      const sourceFile = sourceFileCache.get(specifier);
      if (sourceFile) {
        return {
          getText(start, end) {
            return sourceFile.text.substring(start, end);
          },
          getLength() {
            return sourceFile.text.length;
          },
          getChangeRange() {
            return undefined;
          },
        };
      }
      const version = host.getScriptVersion(specifier);
      if (version != null) {
        return new ScriptSnapshot(specifier, version);
      }
      return undefined;
    },
  };

  /** @type {Array<[string, number]>} */
  const stats = [];
  let statsStart = 0;

  function performanceStart() {
    stats.length = 0;
    statsStart = Date.now();
    ts.performance.enable();
  }

  /**
   * @param {{ program: ts.Program | ts.EmitAndSemanticDiagnosticsBuilderProgram, fileCount?: number }} options
   */
  function performanceProgram({ program, fileCount }) {
    if (program) {
      if ("getProgram" in program) {
        program = program.getProgram();
      }
      stats.push(["Files", program.getSourceFiles().length]);
      stats.push(["Nodes", program.getNodeCount()]);
      stats.push(["Identifiers", program.getIdentifierCount()]);
      stats.push(["Symbols", program.getSymbolCount()]);
      stats.push(["Types", program.getTypeCount()]);
      stats.push(["Instantiations", program.getInstantiationCount()]);
    } else if (fileCount != null) {
      stats.push(["Files", fileCount]);
    }
    const programTime = ts.performance.getDuration("Program");
    const bindTime = ts.performance.getDuration("Bind");
    const checkTime = ts.performance.getDuration("Check");
    const emitTime = ts.performance.getDuration("Emit");
    stats.push(["Parse time", programTime]);
    stats.push(["Bind time", bindTime]);
    stats.push(["Check time", checkTime]);
    stats.push(["Emit time", emitTime]);
    stats.push(
      ["Total TS time", programTime + bindTime + checkTime + emitTime],
    );
  }

  function performanceEnd() {
    const duration = Date.now() - statsStart;
    stats.push(["Compile time", duration]);
    return stats;
  }

  /**
   * @typedef {object} Request
   * @property {Record<string, any>} config
   * @property {boolean} debug
   * @property {string[]} rootNames
   */

  /**
   * Checks the normalized version of the root name and stores it in
   * `normalizedToOriginalMap`. If the normalized specifier is already
   * registered for the different root name, it throws an AssertionError.
   *
   * @param {string} rootName
   */
  function checkNormalizedPath(rootName) {
    const normalized = ts.normalizePath(rootName);
    const originalRootName = normalizedToOriginalMap.get(normalized);
    if (typeof originalRootName === "undefined") {
      normalizedToOriginalMap.set(normalized, rootName);
    } else if (originalRootName !== rootName) {
      // The different root names are normalizd to the same path.
      // This will cause problem when looking up the source for each.
      throw new AssertionError(
        `The different names for the same normalized specifier are specified: normalized=${normalized}, rootNames=${originalRootName},${rootName}`,
      );
    }
  }

  /** The API that is called by Rust when executing a request.
   * @param {Request} request
   */
  function exec({ config, debug: debugFlag, rootNames }) {
    setLogDebug(debugFlag, "TS");
    performanceStart();
    debug(">>> exec start", { rootNames });
    debug(config);

    rootNames.forEach(checkNormalizedPath);

    const { options, errors: configFileParsingDiagnostics } = ts
      .convertCompilerOptionsFromJson(config, "");
    // The `allowNonTsExtensions` is a "hidden" compiler option used in VSCode
    // which is not allowed to be passed in JSON, we need it to allow special
    // URLs which Deno supports. So we need to either ignore the diagnostic, or
    // inject it ourselves.
    Object.assign(options, { allowNonTsExtensions: true });
    const program = ts.createIncrementalProgram({
      rootNames,
      options,
      host,
      configFileParsingDiagnostics,
    });

    const { diagnostics: emitDiagnostics } = program.emit();

    const diagnostics = [
      ...program.getConfigFileParsingDiagnostics(),
      ...program.getSyntacticDiagnostics(),
      ...program.getOptionsDiagnostics(),
      ...program.getGlobalDiagnostics(),
      ...program.getSemanticDiagnostics(),
      ...emitDiagnostics,
    ].filter(({ code }) => !IGNORED_DIAGNOSTICS.includes(code));
    performanceProgram({ program });

    core.opSync("op_respond", {
      diagnostics: fromTypeScriptDiagnostic(diagnostics),
      stats: performanceEnd(),
    });
    debug("<<< exec stop");
  }

  /**
   * @param {number} id
   * @param {any} data
   */
  function respond(id, data = null) {
    core.opSync("op_respond", { id, data });
  }

  /**
   * @param {LanguageServerRequest} request
   */
  function serverRequest({ id, ...request }) {
    debug(`serverRequest()`, { id, ...request });
    // reset all memoized source files names
    scriptFileNamesCache = undefined;
    // evict all memoized source file versions
    scriptVersionCache.clear();
    switch (request.method) {
      case "configure": {
        const { options, errors } = ts
          .convertCompilerOptionsFromJson(request.compilerOptions, "");
        Object.assign(options, { allowNonTsExtensions: true });
        if (errors.length) {
          debug(ts.formatDiagnostics(errors, host));
        }
        compilationSettings = options;
        return respond(id, true);
      }
      case "findRenameLocations": {
        return respond(
          id,
          languageService.findRenameLocations(
            request.specifier,
            request.position,
            request.findInStrings,
            request.findInComments,
            request.providePrefixAndSuffixTextForRename,
          ),
        );
      }
      case "getAsset": {
        const sourceFile = host.getSourceFile(
          request.specifier,
          ts.ScriptTarget.ESNext,
        );
        return respond(id, sourceFile && sourceFile.text);
      }
      case "getApplicableRefactors": {
        return respond(
          id,
          languageService.getApplicableRefactors(
            request.specifier,
            request.range,
            {
              quotePreference: "double",
              allowTextChangesInNewFiles: true,
              provideRefactorNotApplicableReason: true,
            },
            undefined,
            request.kind,
          ),
        );
      }
      case "getEditsForRefactor": {
        return respond(
          id,
          languageService.getEditsForRefactor(
            request.specifier,
            {
              indentSize: 2,
              indentStyle: ts.IndentStyle.Smart,
              semicolons: ts.SemicolonPreference.Insert,
              convertTabsToSpaces: true,
              insertSpaceBeforeAndAfterBinaryOperators: true,
              insertSpaceAfterCommaDelimiter: true,
            },
            request.range,
            request.refactorName,
            request.actionName,
            {
              quotePreference: "double",
            },
          ),
        );
      }
      case "getCodeFixes": {
        return respond(
          id,
          languageService.getCodeFixesAtPosition(
            request.specifier,
            request.startPosition,
            request.endPosition,
            request.errorCodes.map((v) => Number(v)),
            {
              indentSize: 2,
              indentStyle: ts.IndentStyle.Block,
              semicolons: ts.SemicolonPreference.Insert,
            },
            {
              quotePreference: "double",
            },
          ),
        );
      }
      case "getCombinedCodeFix": {
        return respond(
          id,
          languageService.getCombinedCodeFix(
            {
              type: "file",
              fileName: request.specifier,
            },
            request.fixId,
            {
              indentSize: 2,
              indentStyle: ts.IndentStyle.Block,
              semicolons: ts.SemicolonPreference.Insert,
            },
            {
              quotePreference: "double",
            },
          ),
        );
      }
      case "getCompletionDetails": {
        debug("request", request);
        return respond(
          id,
          languageService.getCompletionEntryDetails(
            request.args.specifier,
            request.args.position,
            request.args.name,
            undefined,
            request.args.source,
            undefined,
            // @ts-expect-error this exists in 4.3 but not part of the d.ts
            request.args.data,
          ),
        );
      }
      case "getCompletions": {
        return respond(
          id,
          languageService.getCompletionsAtPosition(
            request.specifier,
            request.position,
            request.preferences,
          ),
        );
      }
      case "getDefinition": {
        return respond(
          id,
          languageService.getDefinitionAndBoundSpan(
            request.specifier,
            request.position,
          ),
        );
      }
      case "getDiagnostics": {
        try {
          /** @type {Record<string, any[]>} */
          const diagnosticMap = {};
          for (const specifier of request.specifiers) {
            diagnosticMap[specifier] = fromTypeScriptDiagnostic([
              ...languageService.getSemanticDiagnostics(specifier),
              ...languageService.getSuggestionDiagnostics(specifier),
              ...languageService.getSyntacticDiagnostics(specifier),
            ].filter(({ code }) => !IGNORED_DIAGNOSTICS.includes(code)));
          }
          return respond(id, diagnosticMap);
        } catch (e) {
          if (!(e instanceof OperationCanceledError)) {
            if ("stack" in e) {
              error(e.stack);
            } else {
              error(e);
            }
          }
          return respond(id, {});
        }
      }
      case "getDocumentHighlights": {
        return respond(
          id,
          languageService.getDocumentHighlights(
            request.specifier,
            request.position,
            request.filesToSearch,
          ),
        );
      }
      case "getEncodedSemanticClassifications": {
        return respond(
          id,
          languageService.getEncodedSemanticClassifications(
            request.specifier,
            request.span,
            ts.SemanticClassificationFormat.TwentyTwenty,
          ),
        );
      }
      case "getImplementation": {
        return respond(
          id,
          languageService.getImplementationAtPosition(
            request.specifier,
            request.position,
          ),
        );
      }
      case "getNavigateToItems": {
        return respond(
          id,
          languageService.getNavigateToItems(
            request.search,
            request.maxResultCount,
            request.fileName,
          ),
        );
      }
      case "getNavigationTree": {
        return respond(
          id,
          languageService.getNavigationTree(request.specifier),
        );
      }
      case "getOutliningSpans": {
        return respond(
          id,
          languageService.getOutliningSpans(
            request.specifier,
          ),
        );
      }
      case "getQuickInfo": {
        return respond(
          id,
          languageService.getQuickInfoAtPosition(
            request.specifier,
            request.position,
          ),
        );
      }
      case "getReferences": {
        return respond(
          id,
          languageService.getReferencesAtPosition(
            request.specifier,
            request.position,
          ),
        );
      }
      case "getSignatureHelpItems": {
        return respond(
          id,
          languageService.getSignatureHelpItems(
            request.specifier,
            request.position,
            request.options,
          ),
        );
      }
      case "getSmartSelectionRange": {
        return respond(
          id,
          languageService.getSmartSelectionRange(
            request.specifier,
            request.position,
          ),
        );
      }
      case "getSupportedCodeFixes": {
        return respond(
          id,
          ts.getSupportedCodeFixes(),
        );
      }
      case "getTypeDefinition": {
        return respond(
          id,
          languageService.getTypeDefinitionAtPosition(
            request.specifier,
            request.position,
          ),
        );
      }
      case "prepareCallHierarchy": {
        return respond(
          id,
          languageService.prepareCallHierarchy(
            request.specifier,
            request.position,
          ),
        );
      }
      case "provideCallHierarchyIncomingCalls": {
        return respond(
          id,
          languageService.provideCallHierarchyIncomingCalls(
            request.specifier,
            request.position,
          ),
        );
      }
      case "provideCallHierarchyOutgoingCalls": {
        return respond(
          id,
          languageService.provideCallHierarchyOutgoingCalls(
            request.specifier,
            request.position,
          ),
        );
      }
      default:
        throw new TypeError(
          // @ts-ignore exhausted case statement sets type to never
          `Invalid request method for request: "${request.method}" (${id})`,
        );
    }
  }

  /** @param {{ debug: boolean; rootUri?: string; }} init */
  function serverInit({ debug: debugFlag, rootUri }) {
    if (hasStarted) {
      throw new Error("The language server has already been initialized.");
    }
    hasStarted = true;
    cwd = rootUri;
    languageService = ts.createLanguageService(host);
    setLogDebug(debugFlag, "TSLS");
    debug("serverInit()");
  }

  let hasStarted = false;

  /** Startup the runtime environment, setting various flags.
   * @param {{ debugFlag?: boolean; legacyFlag?: boolean; }} msg
   */
  function startup({ debugFlag = false }) {
    if (hasStarted) {
      throw new Error("The compiler runtime already started.");
    }
    hasStarted = true;
    setLogDebug(!!debugFlag, "TS");
  }

  // A build time only op that provides some setup information that is used to
  // ensure the snapshot is setup properly.
  /** @type {{ buildSpecifier: string; libs: string[] }} */
  const { buildSpecifier, libs } = core.opSync("op_build_info", {});
  for (const lib of libs) {
    const specifier = `lib.${lib}.d.ts`;
    // we are using internal APIs here to "inject" our custom libraries into
    // tsc, so things like `"lib": [ "deno.ns" ]` are supported.
    if (!ts.libs.includes(lib)) {
      ts.libs.push(lib);
      ts.libMap.set(lib, `lib.${lib}.d.ts`);
    }
    // we are caching in memory common type libraries that will be re-used by
    // tsc on when the snapshot is restored
    assert(
      host.getSourceFile(`${ASSETS}${specifier}`, ts.ScriptTarget.ESNext),
    );
  }
  // this helps ensure as much as possible is in memory that is re-usable
  // before the snapshotting is done, which helps unsure fast "startup" for
  // subsequent uses of tsc in Deno.
  const TS_SNAPSHOT_PROGRAM = ts.createProgram({
    rootNames: [buildSpecifier],
    options: SNAPSHOT_COMPILE_OPTIONS,
    host,
  });
  ts.getPreEmitDiagnostics(TS_SNAPSHOT_PROGRAM);

  // exposes the two functions that are called by `tsc::exec()` when type
  // checking TypeScript.
  globalThis.startup = startup;
  globalThis.exec = exec;

  // exposes the functions that are called when the compiler is used as a
  // language service.
  globalThis.serverInit = serverInit;
  globalThis.serverRequest = serverRequest;
})(this);
