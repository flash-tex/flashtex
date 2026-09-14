#!/usr/bin/env python3
"""Writes apps/ios/FlashTeXPad.xcodeproj deterministically (no third-party tools).

Targets:
  FlashTeXPad         iPadOS 17+ SwiftUI app; links the local package products
                      FlashTeXPadKit, NearbyClient, FlashTeXProtocol
                      (Packages/FlashTeXPadKit -> symlinks into apps/mac).
  FlashTeXPadTests    XCTest unit bundle hosted in the app (FakeMac round trip).
  FlashTeXPadUITests  XCUITest bundle (opens the sample, drives the review gate).

Re-run after adding a source file:  python3 apps/ios/scripts/generate-xcodeproj.py
The output is byte-stable for the same inputs, so `git diff` shows only real changes.
"""
import hashlib
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROJ = os.path.join(ROOT, "FlashTeXPad.xcodeproj")
PACKAGE_PATH = "Packages/FlashTeXPadKit"
DEPLOYMENT_TARGET = "17.0"
BUNDLE_PREFIX = "tech.jay3332.flashtex"


def oid(name):
    """24 upper-hex chars, stable per logical name."""
    return hashlib.sha1(name.encode()).hexdigest()[:24].upper()


def sources(rel_dir, exts=(".swift",)):
    d = os.path.join(ROOT, rel_dir)
    return sorted(f for f in os.listdir(d) if f.endswith(exts) and not f.startswith("."))


def q(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


class Project:
    def __init__(self):
        self.objects = []  # (id, isa, text)

    def add(self, id_, isa, body, comment=None):
        c = f" /* {comment} */" if comment else ""
        self.objects.append((id_, isa, f"\t\t{id_}{c} = {{\n\t\t\tisa = {isa};\n{body}\t\t}};\n"))

    def render(self, root_id):
        out = ["// !$*UTF8*$!\n{\n\tarchiveVersion = 1;\n\tclasses = {\n\t};\n\tobjectVersion = 60;\n\tobjects = {\n"]
        by_isa = {}
        for id_, isa, text in self.objects:
            by_isa.setdefault(isa, []).append(text)
        for isa in sorted(by_isa):
            out.append(f"\n/* Begin {isa} section */\n")
            out.extend(by_isa[isa])
            out.append(f"/* End {isa} section */\n")
        out.append(f"\t}};\n\trootObject = {root_id} /* Project object */;\n}}\n")
        return "".join(out)


def build_settings(d):
    return "".join(f"\t\t\t\t{k} = {v};\n" for k, v in sorted(d.items()))


def main():
    p = Project()
    ids = {}

    def file_ref(path, name=None, ftype=None, tree="<group>"):
        name = name or os.path.basename(path)
        id_ = oid("ref:" + path)
        ext = os.path.splitext(name)[1]
        ftype = ftype or {".swift": "sourcecode.swift", ".plist": "text.plist.xml", ".json": "text.json",
                          ".tex": "text", ".md": "net.daringfireball.markdown"}.get(ext, "text")
        p.add(id_, "PBXFileReference",
              f"\t\t\tlastKnownFileType = {ftype};\n\t\t\tpath = {q(name)};\n\t\t\tsourceTree = {q(tree)};\n", name)
        return id_

    def build_file(ref_id, path, phase):
        id_ = oid(f"build:{phase}:{path}")
        p.add(id_, "PBXBuildFile", f"\t\t\tfileRef = {ref_id};\n", f"{os.path.basename(path)} in {phase}")
        return id_

    def group(name, path, children, id_name=None):
        id_ = oid("group:" + (id_name or path or name))
        kids = "".join(f"\t\t\t\t{c},\n" for c in children)
        body = f"\t\t\tchildren = (\n{kids}\t\t\t);\n"
        if path:
            body += f"\t\t\tpath = {q(path)};\n"
        body += "\t\t\tsourceTree = \"<group>\";\n"
        p.add(id_, "PBXGroup", body, name)
        return id_

    # --- package reference + product dependencies
    pkg_ref = oid("pkg:" + PACKAGE_PATH)
    p.add(pkg_ref, "XCLocalSwiftPackageReference", f"\t\t\trelativePath = {PACKAGE_PATH};\n",
          f'XCLocalSwiftPackageReference "{PACKAGE_PATH}"')

    def product_dep(target, product):
        id_ = oid(f"productdep:{target}:{product}")
        p.add(id_, "XCSwiftPackageProductDependency", f"\t\t\tpackage = {pkg_ref} /* XCLocalSwiftPackageReference \"{PACKAGE_PATH}\" */;\n\t\t\tproductName = {product};\n", product)
        bf = oid(f"productbuild:{target}:{product}")
        p.add(bf, "PBXBuildFile", f"\t\t\tproductRef = {id_} /* {product} */;\n", f"{product} in Frameworks")
        return id_, bf

    # --- targets
    def target(name, product_type, group_dir, src_files, res_files, settings, deps=(), products=(), extra_deps_on=()):
        src_refs = [(f, file_ref(f"{group_dir}/{f}", name=f)) for f in src_files]
        res_refs = [(f, file_ref(f"{group_dir}/{f}", name=f)) for f in res_files]
        plist_ref = file_ref(f"{group_dir}/Info.plist", name="Info.plist") if os.path.exists(os.path.join(ROOT, group_dir, "Info.plist")) else None
        kids = [r for _, r in src_refs] + [r for _, r in res_refs] + ([plist_ref] if plist_ref else [])
        grp = group(group_dir, group_dir, kids)

        src_phase = oid(f"phase:sources:{name}")
        files = "".join(f"\t\t\t\t{build_file(r, f'{group_dir}/{f}', 'Sources')},\n" for f, r in src_refs)
        p.add(src_phase, "PBXSourcesBuildPhase",
              f"\t\t\tbuildActionMask = 2147483647;\n\t\t\tfiles = (\n{files}\t\t\t);\n\t\t\trunOnlyForDeploymentPostprocessing = 0;\n", "Sources")

        res_phase = oid(f"phase:resources:{name}")
        files = "".join(f"\t\t\t\t{build_file(r, f'{group_dir}/{f}', 'Resources')},\n" for f, r in res_refs)
        p.add(res_phase, "PBXResourcesBuildPhase",
              f"\t\t\tbuildActionMask = 2147483647;\n\t\t\tfiles = (\n{files}\t\t\t);\n\t\t\trunOnlyForDeploymentPostprocessing = 0;\n", "Resources")

        fw_phase = oid(f"phase:frameworks:{name}")
        prod_ids = [product_dep(name, prod) for prod in products]
        files = "".join(f"\t\t\t\t{bf},\n" for _, bf in prod_ids)
        p.add(fw_phase, "PBXFrameworksBuildPhase",
              f"\t\t\tbuildActionMask = 2147483647;\n\t\t\tfiles = (\n{files}\t\t\t);\n\t\t\trunOnlyForDeploymentPostprocessing = 0;\n", "Frameworks")

        ext = {"com.apple.product-type.application": "app"}.get(product_type, "xctest")
        product_ref = oid(f"product:{name}")
        p.add(product_ref, "PBXFileReference",
              f"\t\t\texplicitFileType = {q('wrapper.application' if ext == 'app' else 'wrapper.cfbundle')};\n\t\t\tincludeInIndex = 0;\n\t\t\tpath = {q(name + '.' + ext)};\n\t\t\tsourceTree = BUILT_PRODUCTS_DIR;\n", f"{name}.{ext}")

        confs = []
        for cfg in ("Debug", "Release"):
            cid = oid(f"conf:{name}:{cfg}")
            s = dict(settings)
            if plist_ref:
                s["INFOPLIST_FILE"] = f"{group_dir}/Info.plist"
            p.add(cid, "XCBuildConfiguration", f"\t\t\tbuildSettings = {{\n{build_settings(s)}\t\t\t}};\n\t\t\tname = {cfg};\n", cfg)
            confs.append(cid)
        clist = oid(f"conflist:{name}")
        p.add(clist, "XCConfigurationList",
              "\t\t\tbuildConfigurations = (\n" + "".join(f"\t\t\t\t{c},\n" for c in confs) + "\t\t\t);\n\t\t\tdefaultConfigurationIsVisible = 0;\n\t\t\tdefaultConfigurationName = Release;\n",
              f'Build configuration list for PBXNativeTarget "{name}"')

        dep_ids = []
        for dep_target in extra_deps_on:
            proxy = oid(f"proxy:{name}:{dep_target}")
            p.add(proxy, "PBXContainerItemProxy",
                  f"\t\t\tcontainerPortal = {ids['project']} /* Project object */;\n\t\t\tproxyType = 1;\n\t\t\tremoteGlobalIDString = {ids['target:' + dep_target]};\n\t\t\tremoteInfo = {dep_target};\n", "PBXContainerItemProxy")
            d = oid(f"dep:{name}:{dep_target}")
            p.add(d, "PBXTargetDependency", f"\t\t\ttarget = {ids['target:' + dep_target]} /* {dep_target} */;\n\t\t\ttargetProxy = {proxy} /* PBXContainerItemProxy */;\n", "PBXTargetDependency")
            dep_ids.append(d)

        tid = oid(f"target:{name}")
        ids[f"target:{name}"] = tid
        body = (f"\t\t\tbuildConfigurationList = {clist} /* Build configuration list for PBXNativeTarget \"{name}\" */;\n"
                f"\t\t\tbuildPhases = (\n\t\t\t\t{src_phase} /* Sources */,\n\t\t\t\t{fw_phase} /* Frameworks */,\n\t\t\t\t{res_phase} /* Resources */,\n\t\t\t);\n"
                f"\t\t\tbuildRules = (\n\t\t\t);\n"
                f"\t\t\tdependencies = (\n" + "".join(f"\t\t\t\t{d},\n" for d in dep_ids) + "\t\t\t);\n"
                f"\t\t\tname = {name};\n"
                f"\t\t\tpackageProductDependencies = (\n" + "".join(f"\t\t\t\t{pid} /* {prod} */,\n" for (pid, _), prod in zip(prod_ids, products)) + "\t\t\t);\n"
                f"\t\t\tproductName = {name};\n"
                f"\t\t\tproductReference = {product_ref} /* {name}.{ext} */;\n"
                f"\t\t\tproductType = {q(product_type)};\n")
        p.add(tid, "PBXNativeTarget", body, name)
        return tid, grp, product_ref

    ids["project"] = oid("project")
    common = {
        "ALWAYS_SEARCH_USER_PATHS": "NO",
        "CLANG_ENABLE_MODULES": "YES",
        "CODE_SIGN_IDENTITY": q("-"),
        "CODE_SIGN_STYLE": "Manual",
        "CURRENT_PROJECT_VERSION": "1",
        "DEVELOPMENT_TEAM": q(""),
        "ENABLE_USER_SCRIPT_SANDBOXING": "YES",
        "IPHONEOS_DEPLOYMENT_TARGET": DEPLOYMENT_TARGET,
        "MARKETING_VERSION": "0.1",
        "PRODUCT_NAME": q("$(TARGET_NAME)"),
        "SDKROOT": "iphoneos",
        "SWIFT_EMIT_LOC_STRINGS": "YES",
        "SWIFT_STRICT_CONCURRENCY": "minimal",
        "SWIFT_VERSION": "5.0",
        "TARGETED_DEVICE_FAMILY": q("2"),
    }
    app_settings = dict(common, **{
        "ASSETCATALOG_COMPILER_APPICON_NAME": "AppIcon",
        "GENERATE_INFOPLIST_FILE": "NO",
        "LD_RUNPATH_SEARCH_PATHS": q("$(inherited) @executable_path/Frameworks"),
        "PRODUCT_BUNDLE_IDENTIFIER": f"{BUNDLE_PREFIX}.FlashTeXPad",
        "SUPPORTS_MACCATALYST": "NO",
    })
    app_dir = "FlashTeXPad"
    app_tid, app_grp, app_prod = target(
        "FlashTeXPad", "com.apple.product-type.application", app_dir,
        sources(app_dir), ["Resources/" + f for f in sources(app_dir + "/Resources", (".tex", ".json", ".png"))],
        app_settings, products=("FlashTeXPadKit", "NearbyClient", "FlashTeXProtocol"))

    unit_settings = dict(common, **{
        "BUNDLE_LOADER": q("$(TEST_HOST)"),
        "GENERATE_INFOPLIST_FILE": "YES",
        "LD_RUNPATH_SEARCH_PATHS": q("$(inherited) @executable_path/Frameworks @loader_path/Frameworks"),
        "PRODUCT_BUNDLE_IDENTIFIER": f"{BUNDLE_PREFIX}.FlashTeXPadTests",
        "TEST_HOST": q("$(BUILT_PRODUCTS_DIR)/FlashTeXPad.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/FlashTeXPad"),
    })
    unit_tid, unit_grp, unit_prod = target(
        "FlashTeXPadTests", "com.apple.product-type.bundle.unit-test", "FlashTeXPadTests",
        sources("FlashTeXPadTests"), [], unit_settings, products=("FlashTeXPadKit", "NearbyClient", "FlashTeXProtocol"),
        extra_deps_on=("FlashTeXPad",))

    ui_settings = dict(common, **{
        "GENERATE_INFOPLIST_FILE": "YES",
        "LD_RUNPATH_SEARCH_PATHS": q("$(inherited) @executable_path/Frameworks @loader_path/Frameworks"),
        "PRODUCT_BUNDLE_IDENTIFIER": f"{BUNDLE_PREFIX}.FlashTeXPadUITests",
        "TEST_TARGET_NAME": "FlashTeXPad",
    })
    ui_tid, ui_grp, ui_prod = target(
        "FlashTeXPadUITests", "com.apple.product-type.bundle.ui-testing", "FlashTeXPadUITests",
        sources("FlashTeXPadUITests"), [], ui_settings, products=("NearbyClient",), extra_deps_on=("FlashTeXPad",))

    products_grp = group("Products", None, [app_prod, unit_prod, ui_prod], id_name="Products")
    pkg_dir_ref = oid("ref:" + PACKAGE_PATH)
    p.add(pkg_dir_ref, "PBXFileReference", f"\t\t\tlastKnownFileType = folder;\n\t\t\tpath = {q(PACKAGE_PATH)};\n\t\t\tsourceTree = \"<group>\";\n", PACKAGE_PATH)
    main_grp = group("", None, [app_grp, unit_grp, ui_grp, pkg_dir_ref, products_grp], id_name="main")

    proj_confs = []
    for cfg in ("Debug", "Release"):
        cid = oid(f"conf:project:{cfg}")
        s = {"ENABLE_TESTABILITY": "YES" if cfg == "Debug" else "NO",
             "SWIFT_OPTIMIZATION_LEVEL": q("-Onone") if cfg == "Debug" else q("-O"),
             "DEBUG_INFORMATION_FORMAT": "dwarf" if cfg == "Debug" else q("dwarf-with-dsym"),
             "SWIFT_ACTIVE_COMPILATION_CONDITIONS": "DEBUG" if cfg == "Debug" else q(""),
             "ONLY_ACTIVE_ARCH": "YES" if cfg == "Debug" else "NO",
             "IPHONEOS_DEPLOYMENT_TARGET": DEPLOYMENT_TARGET}
        p.add(cid, "XCBuildConfiguration", f"\t\t\tbuildSettings = {{\n{build_settings(s)}\t\t\t}};\n\t\t\tname = {cfg};\n", cfg)
        proj_confs.append(cid)
    proj_clist = oid("conflist:project")
    p.add(proj_clist, "XCConfigurationList",
          "\t\t\tbuildConfigurations = (\n" + "".join(f"\t\t\t\t{c},\n" for c in proj_confs) + "\t\t\t);\n\t\t\tdefaultConfigurationIsVisible = 0;\n\t\t\tdefaultConfigurationName = Release;\n",
          'Build configuration list for PBXProject "FlashTeXPad"')

    p.add(ids["project"], "PBXProject",
          f"\t\t\tattributes = {{\n\t\t\t\tBuildIndependentTargetsInParallel = 1;\n\t\t\t\tLastSwiftUpdateCheck = 1630;\n\t\t\t\tLastUpgradeCheck = 1630;\n"
          f"\t\t\t\tTargetAttributes = {{\n\t\t\t\t\t{unit_tid} = {{\n\t\t\t\t\t\tTestTargetID = {app_tid};\n\t\t\t\t\t}};\n\t\t\t\t\t{ui_tid} = {{\n\t\t\t\t\t\tTestTargetID = {app_tid};\n\t\t\t\t\t}};\n\t\t\t\t}};\n\t\t\t}};\n"
          f"\t\t\tbuildConfigurationList = {proj_clist};\n\t\t\tcompatibilityVersion = \"Xcode 15.0\";\n\t\t\tdevelopmentRegion = en;\n\t\t\thasScannedForEncodings = 0;\n"
          f"\t\t\tknownRegions = (\n\t\t\t\ten,\n\t\t\t\tBase,\n\t\t\t);\n\t\t\tmainGroup = {main_grp};\n"
          f"\t\t\tpackageReferences = (\n\t\t\t\t{pkg_ref} /* XCLocalSwiftPackageReference \"{PACKAGE_PATH}\" */,\n\t\t\t);\n"
          f"\t\t\tproductRefGroup = {products_grp} /* Products */;\n\t\t\tprojectDirPath = \"\";\n\t\t\tprojectRoot = \"\";\n"
          f"\t\t\ttargets = (\n\t\t\t\t{app_tid} /* FlashTeXPad */,\n\t\t\t\t{unit_tid} /* FlashTeXPadTests */,\n\t\t\t\t{ui_tid} /* FlashTeXPadUITests */,\n\t\t\t);\n",
          "Project object")

    os.makedirs(os.path.join(PROJ, "xcshareddata", "xcschemes"), exist_ok=True)
    with open(os.path.join(PROJ, "project.pbxproj"), "w") as f:
        f.write(p.render(ids["project"]))

    scheme = f"""<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion = "1630" version = "1.7">
   <BuildAction parallelizeBuildables = "YES" buildImplicitDependencies = "YES">
      <BuildActionEntries>
         <BuildActionEntry buildForTesting = "YES" buildForRunning = "YES" buildForProfiling = "YES" buildForArchiving = "YES" buildForAnalyzing = "YES">
            <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "{app_tid}" BuildableName = "FlashTeXPad.app" BlueprintName = "FlashTeXPad" ReferencedContainer = "container:FlashTeXPad.xcodeproj"/>
         </BuildActionEntry>
      </BuildActionEntries>
   </BuildAction>
   <TestAction buildConfiguration = "Debug" selectedDebuggerIdentifier = "Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier = "Xcode.DebuggerFoundation.Launcher.LLDB" shouldUseLaunchSchemeArgsEnv = "YES">
      <Testables>
         <TestableReference skipped = "NO">
            <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "{unit_tid}" BuildableName = "FlashTeXPadTests.xctest" BlueprintName = "FlashTeXPadTests" ReferencedContainer = "container:FlashTeXPad.xcodeproj"/>
            <!-- Hosted in the app: without these the production PadModel can
                 auto-reconnect from a leftover Keychain pairing and race the
                 XCTest PadModel (observed EXC_BAD_ACCESS in pollOutcome on
                 Xcode 26.6 / iOS 26.5). UI tests pass their own args. -->
            <CommandLineArguments>
               <CommandLineArgument argument = "-flashtexpad-fresh" isEnabled = "YES"/>
               <CommandLineArgument argument = "-flashtexpad-no-autoreconnect" isEnabled = "YES"/>
            </CommandLineArguments>
         </TestableReference>
         <TestableReference skipped = "NO">
            <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "{ui_tid}" BuildableName = "FlashTeXPadUITests.xctest" BlueprintName = "FlashTeXPadUITests" ReferencedContainer = "container:FlashTeXPad.xcodeproj"/>
         </TestableReference>
      </Testables>
   </TestAction>
   <LaunchAction buildConfiguration = "Debug" selectedDebuggerIdentifier = "Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier = "Xcode.DebuggerFoundation.Launcher.LLDB" launchStyle = "0" useCustomWorkingDirectory = "NO" ignoresPersistentStateOnLaunch = "NO" debugDocumentVersioning = "YES" debugServiceExtension = "internal" allowLocationSimulation = "YES">
      <BuildableProductRunnable runnableDebuggingMode = "0">
         <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "{app_tid}" BuildableName = "FlashTeXPad.app" BlueprintName = "FlashTeXPad" ReferencedContainer = "container:FlashTeXPad.xcodeproj"/>
      </BuildableProductRunnable>
   </LaunchAction>
   <ProfileAction buildConfiguration = "Release" shouldUseLaunchSchemeArgsEnv = "YES" savedToolIdentifier = "" useCustomWorkingDirectory = "NO" debugDocumentVersioning = "YES">
      <BuildableProductRunnable runnableDebuggingMode = "0">
         <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "{app_tid}" BuildableName = "FlashTeXPad.app" BlueprintName = "FlashTeXPad" ReferencedContainer = "container:FlashTeXPad.xcodeproj"/>
      </BuildableProductRunnable>
   </ProfileAction>
   <AnalyzeAction buildConfiguration = "Debug"/>
   <ArchiveAction buildConfiguration = "Release" revealArchiveInOrganizer = "YES"/>
</Scheme>
"""
    with open(os.path.join(PROJ, "xcshareddata", "xcschemes", "FlashTeXPad.xcscheme"), "w") as f:
        f.write(scheme)
    print(f"wrote {PROJ}", file=sys.stderr)


if __name__ == "__main__":
    main()
