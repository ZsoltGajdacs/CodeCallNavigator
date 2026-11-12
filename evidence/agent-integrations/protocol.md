# Real compiler MCP probe

2026-11-12; Node 24.15.0, TypeScript 6.0.3; packaged engine and MCP, JSON-line stdio.

```json
[
  {
    "tool": "graph_status",
    "arguments": {},
    "milliseconds": 4.51,
    "bytes": 304,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI error graph_missing: No such file or directory (os error 2)"
        }
      ],
      "structuredContent": {
        "error": {
          "code": "graph_missing",
          "message": "No such file or directory (os error 2)"
        },
        "freshness": null,
        "generation": null,
        "result": null,
        "version": 1
      },
      "isError": true
    }
  },
  {
    "tool": "discover_builds",
    "arguments": {},
    "milliseconds": 3.82,
    "bytes": 532,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "default_selection": [
          {
            "kind": "typescript",
            "path": "/private<temporary-project>/tsconfig.json"
          }
        ],
        "targets": [
          {
            "kind": "typescript",
            "label": "agent-graph-io2met80 / tsconfig.json",
            "language": "typescript",
            "path": "/private<temporary-project>/tsconfig.json"
          }
        ]
      },
      "isError": false
    }
  },
  {
    "tool": "refresh_graph",
    "arguments": {},
    "milliseconds": 644.33,
    "bytes": 29336,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": null,
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "engine_versions": "{\"callatlas\":{\"legacy_patch\":\"patches/callatlas-assembly-identity-legacy.patch\",\"legacy_patch_sha256\":\"ec723795ee841b98c2efbf49a7f010cd3f22652889c0e5c34866461b0137c574\",\"older_patch\":\"patches/callatlas-assembly-identity-entries.patch\",\"older_patch_sha256\":\"aeb8b8b664e21331e7815466746ec482a63407e4ae05ce8275e2810f552ff405\",\"patch\":\"patches/callatlas-assembly-identity.patch\",\"patch_sha256\":\"7faebb71f4a998f62bac10bd44741169bfd412ad0f006fbd4698d8d8c58b562b\",\"pre_framework_patch\":\"patches/callatlas-assembly-identity-pre-framework.patch\",\"pre_framework_patch_sha256\":\"7b64792f14f02fafb4472402b0ab26ac53d9be6e496ef73964ad49791db1d08a\",\"pre_http_patch\":\"patches/callatlas-assembly-identity-pre-http.patch\",\"pre_http_patch_sha256\":\"3dc7ede680f323e7e394ecdd5ef9551f90ca1cf5413bed589241b47a7834c216\",\"pre_review_patch\":\"patches/callatlas-assembly-identity-pre-review.patch\",\"pre_review_patch_sha256\":\"874c4dbd8543abb34e6c87b634e4cf3d67acc300b80a66d28e58fbaafe90152e\",\"pre_visibility_patch\":\"patches/callatlas-assembly-identity-pre-visibility.patch\",\"pre_visibility_patch_sha256\":\"c9633858854be41ab7e6867df8727a74de6674363f20636216555ed7cfe7472c\",\"previous_patch\":\"patches/callatlas-assembly-identity-policy.patch\",\"previous_patch_sha256\":\"a3981a0c07ef932a498f58113994c7858bd53242e0f0860355af9df56704e810\",\"repository\":\"https://github.com/castorhrio/callatlas.git\",\"revision\":\"916c2419c0cdb91b5b86b84df4842dadb17bb6b4\"},\"callatlas_framework_facts\":\"1\",\"callatlas_http_facts\":\"1\",\"callatlas_protocol\":\"callatlas.ir.v3\",\"java-callgraph2\":{\"bcel_version\":\"6.13.0\",\"repository\":\"https://github.com/Adrninistrator/java-callgraph2.git\",\"revision\":\"77f566d0878958bab929b4cdeafcc2c762ff555e\",\"version\":\"4.1.0\"},\"projects\":{\"/private<temporary-project>/tsconfig.json\":{\"adapter\":\"6\",\"boundary_rules\":\"1\",\"next:/private<temporary-project>/tsconfig.json\":\"16.4.2\",\"next_rules\":\"1\",\"node\":\"24.15.0\",\"react:/private<temporary-project>/tsconfig.json\":\"19.3.1\",\"react_rules\":\"1\",\"typescript\":\"6.0.3\",\"typescript:/private<temporary-project>/tsconfig.json\":\"6.0.3\"}}}",
          "event": "complete",
          "fingerprints": [
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "0947112b8571fa3276f658586d7c8c50e40b258dc05f848b6d0e166903307ebb",
              "modified_ns": 1792957718748976406,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/next/package.json",
              "size": 3188
            },
            {
              "hash": "4974b7aaa9319a549185409d032bc92427826bc7c4804eed8b24be54d1995c3e",
              "modified_ns": 1792957716891906647,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/react/package.json",
              "size": 1242
            },
            {
              "hash": "dee990117dfbc2b88b7eb05253bfd7fe6b33e60cb1cd4a3c85d80ba5cbe5f031",
              "modified_ns": 1792957717482301508,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.decorators.d.ts",
              "size": 13153
            },
            {
              "hash": "65e3a677b1f0808cc6c676010cee5449abdaf4535997528eb59631ecc42da55d",
              "modified_ns": 1792957717483231625,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.decorators.legacy.d.ts",
              "size": 1287
            },
            {
              "hash": "ad664232e56da1d48c64483a79fe96d704f4459516f80c6d71b6d281541d33bd",
              "modified_ns": 1792957717483840245,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.asynciterable.d.ts",
              "size": 933
            },
            {
              "hash": "6147499cf009f666b611af183db7812800a24369b4c7c27ddd367860bc730790",
              "modified_ns": 1792957717509715785,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.d.ts",
              "size": 2349483
            },
            {
              "hash": "ad664232e56da1d48c64483a79fe96d704f4459516f80c6d71b6d281541d33bd",
              "modified_ns": 1792957717510443237,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.iterable.d.ts",
              "size": 933
            },
            {
              "hash": "8110e907b0282869297b46b8f66980b24bbfbef194ebf04008026c8dc910b21b",
              "modified_ns": 1792957717510913150,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.collection.d.ts",
              "size": 5305
            },
            {
              "hash": "dcd33183cd1eceac1796fe643a1804ed7c4537947488911f433f13be1be64315",
              "modified_ns": 1792957717512083057,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.core.d.ts",
              "size": 22824
            },
            {
              "hash": "ebb9c892e0747f62a3d9e5203876c36f506d89933d6c0134540a7819112a45c9",
              "modified_ns": 1792957717512809301,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.d.ts",
              "size": 1202
            },
            {
              "hash": "e314f64d0d8478d702e429ff2d89dff8737e65a3f4509a3757791eea8c5c7e99",
              "modified_ns": 1792957717513334880,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.generator.d.ts",
              "size": 2518
            },
            {
              "hash": "592ac08b76799d1cfeb707171a88f79fa2f3b51a5c9c2fa65b6d8db9fd5bdb61",
              "modified_ns": 1792957717514120582,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.iterable.d.ts",
              "size": 18185
            },
            {
              "hash": "cc23ff12135871b0b3409a2042d3d9ccecaa3c111e0f73c81eed990e9752d362",
              "modified_ns": 1792957717514852159,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.promise.d.ts",
              "size": 3161
            },
            {
              "hash": "a32ee6a6af28daa75edcd23b25c3231162ac23887baf9a91fa184656acba4908",
              "modified_ns": 1792957717515637945,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.proxy.d.ts",
              "size": 5212
            },
            {
              "hash": "4433220e9ff2107fa4fe1bcaa88ef1682a98669d663a66cf37adbcebb958f67f",
              "modified_ns": 1792957717516614437,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.reflect.d.ts",
              "size": 6454
            },
            {
              "hash": "4ea16d68d5276616ccdec1b716993a8b3bb8207eee025c8f2fbd8d370207eec4",
              "modified_ns": 1792957717516932767,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.symbol.d.ts",
              "size": 1610
            },
            {
              "hash": "987b64e5d8cebc263aa7f1b2de2eb3a64373a61bd444ce73cfd092434408d263",
              "modified_ns": 1792957717517568804,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.symbol.wellknown.d.ts",
              "size": 10971
            },
            {
              "hash": "0ad35396159f585928341f576a1b46aa170a439cfe49278e16b565c792b56bbc",
              "modified_ns": 1792957717525173284,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.array.include.d.ts",
              "size": 5165
            },
            {
              "hash": "75aaeb4453119518c1099ac5c05c76d433cb83ad19a4c89a6f47d89ed2179260",
              "modified_ns": 1792957717525511364,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.d.ts",
              "size": 926
            },
            {
              "hash": "a9b114a2761723978c531b66d27a5285f08c01fe3e538ae0b456c578cf996f81",
              "modified_ns": 1792957717526596730,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.intl.d.ts",
              "size": 1445
            },
            {
              "hash": "89b2d8fe38506fe46cf087c1640126c192b543d59b59644284c3e9240061445e",
              "modified_ns": 1792957717527379307,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.arraybuffer.d.ts",
              "size": 876
            },
            {
              "hash": "851df1d49109faccb894aedec02d5672b42ade3a0fc03c64b7d67c005fb1a670",
              "modified_ns": 1792957717528089802,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.d.ts",
              "size": 1123
            },
            {
              "hash": "40322fc8016bf926f02a930394867f9ae5a74c35f619602568b771a667189ce7",
              "modified_ns": 1792957717528540340,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.date.d.ts",
              "size": 1877
            },
            {
              "hash": "2e77da3823b36098b6ea9f602e4ba0fa03079f5ada9adb2d4dddfe8591e5f443",
              "modified_ns": 1792957717529845746,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.intl.d.ts",
              "size": 1410
            },
            {
              "hash": "8171c1c57b999098bf2b02442a374df2ab108bde7e9f3ded2d3a0779896bb5fb",
              "modified_ns": 1792957717530487740,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.object.d.ts",
              "size": 2434
            },
            {
              "hash": "7f85b7c8b57ca90da2d85e2e52f4e264371f1744d17fee15d413372cf0351f32",
              "modified_ns": 1792957717531288317,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.sharedmemory.d.ts",
              "size": 7170
            },
            {
              "hash": "e0cccda8bc4a5bfd01f7d1c9f007b2251cd768b97335ac3b15a1e3a0f3862ff7",
              "modified_ns": 1792957717532081644,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.string.d.ts",
              "size": 2339
            },
            {
              "hash": "1bb8797ed5460005e848ce6153a49ba0d6d2e9a0985cd3d153752a8713d09df4",
              "modified_ns": 1792957717532369392,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.typedarrays.d.ts",
              "size": 1503
            },
            {
              "hash": "761a0763ae83dae69cb358eddf073bd3f01edbdc467510851e62b2d3ba0fc6b1",
              "modified_ns": 1792957717533056511,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.asyncgenerator.d.ts",
              "size": 2653
            },
            {
              "hash": "5036834ea5402e07381d56ce8d1d58263662380e10102fc832e67fae9236ede9",
              "modified_ns": 1792957717533762130,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.asynciterable.d.ts",
              "size": 2218
            },
            {
              "hash": "1c2eb86a62014e4418b640799b9e85da46c95c0f4ff0a1fec7fe1b77fda53771",
              "modified_ns": 1792957717535207577,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.d.ts",
              "size": 1049
            },
            {
              "hash": "6ca970975088545700c7dff03c96a4e2f0072ae320255fcbe6f9afc09fcfae7c",
              "modified_ns": 1792957717537052395,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.intl.d.ts",
              "size": 2982
            },
            {
              "hash": "ee19ed8ba7b26224ce45dd97f874c7a361c1370b7c812a0585460727c94fcecf",
              "modified_ns": 1792957717537502642,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.promise.d.ts",
              "size": 1314
            },
            {
              "hash": "fb4b7dd4d24b0818898cf50eb81d106c7e4dc0bb5f56f019fcc55318783cc41c",
              "modified_ns": 1792957717537905763,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.regexp.d.ts",
              "size": 1193
            },
            {
              "hash": "a4246a2e5319c6eaa00b5bc0400bb4be35960dc7a6260ff9b2b62d9c5a1880c6",
              "modified_ns": 1792957717538797214,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.array.d.ts",
              "size": 3124
            },
            {
              "hash": "533814bf32853a8ce284a10f674376a6645113f1280d299f947378a3bda42b01",
              "modified_ns": 1792957717539556875,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.d.ts",
              "size": 1032
            },
            {
              "hash": "add61133869c6272e0c25fad9773eb8903511d5bc44b8d13412a708266834618",
              "modified_ns": 1792957717540304410,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.intl.d.ts",
              "size": 919
            },
            {
              "hash": "4dc687f0ba123d1c25b2da24855c4f575982d8f94bfecdbb1a78cf6fbf7c9206",
              "modified_ns": 1792957717540901031,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.object.d.ts",
              "size": 1434
            },
            {
              "hash": "961f733c1224441b118521a287c9b7601fae8631ec5f4eecd18a17fbef8f27be",
              "modified_ns": 1792957717541325902,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.string.d.ts",
              "size": 1489
            },
            {
              "hash": "e5dd064c8f5e7946fefd0a3aa310ed687b42c6103f09b7469f197f4e367973ec",
              "modified_ns": 1792957717541854565,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.symbol.d.ts",
              "size": 967
            },
            {
              "hash": "06c85612d54ca055608b257dd12e0b6649b3aaf9da515a0bab0dc64ac222634d",
              "modified_ns": 1792957717543032013,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.bigint.d.ts",
              "size": 38258
            },
            {
              "hash": "359ae19ab0853e9c5ed05f8d7cbf1ac5274f807b26a7a6909808a0190b7fceab",
              "modified_ns": 1792957717543727883,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.d.ts",
              "size": 1162
            },
            {
              "hash": "8654796f949b33be4c464d898336329f78eea030fd2031dc5fd2ab684e5e2be8",
              "modified_ns": 1792957717544045838,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.date.d.ts",
              "size": 2917
            },
            {
              "hash": "4d2bc55af74f7e40ee234b15415f64de5dfc3b310d47d8616562686b58674f1e",
              "modified_ns": 1792957717545508535,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.intl.d.ts",
              "size": 22399
            },
            {
              "hash": "58a0aee7d9307e467886da0b2543f8484b194843bf8678d9dd819771e61c2cd3",
              "modified_ns": 1792957717546307153,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.number.d.ts",
              "size": 1548
            },
            {
              "hash": "c11c0a3ddb849e910921cf0237bf03ede59453012b9e4ff9a6106f48193df8b1",
              "modified_ns": 1792957717546746775,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.promise.d.ts",
              "size": 1764
            },
            {
              "hash": "d6b0c2a40f02259a8ccaceb25409975d2ddf675bde1cd38b0f4a17d54cdc0c17",
              "modified_ns": 1792957717547637892,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.sharedmemory.d.ts",
              "size": 5089
            },
            {
              "hash": "aba305cace33a1524511f471c385855e04ab4eba415cf05433fa6028c5461df5",
              "modified_ns": 1792957717547890432,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.string.d.ts",
              "size": 2451
            },
            {
              "hash": "8d6a36d523956e0f594f1ca3fdcadf2923fcd2b45072403ae2edf87b62bdb9b3",
              "modified_ns": 1792957717548410095,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.symbol.wellknown.d.ts",
              "size": 1566
            },
            {
              "hash": "6ef27e928c0977a62ce2323d0709db70be4b76c160c446095340d7e4c5d4690f",
              "modified_ns": 1792957717549072256,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.d.ts",
              "size": 997
            },
            {
              "hash": "e5b976405b0b3c8f4e85a1e371c2b91a2ac55a81b93ed34990495e80ee98ac1a",
              "modified_ns": 1792957717550261829,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.intl.d.ts",
              "size": 8206
            },
            {
              "hash": "a3a6fee8121f9153f108329375311f975638efcb83aef20dfef9b2692861fbf4",
              "modified_ns": 1792957717550795909,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.promise.d.ts",
              "size": 2223
            },
            {
              "hash": "4ab97a788f157b3d3015049d8a0b7e2c8c970316dbfbac5e4c615786dc0a93f7",
              "modified_ns": 1792957717551407195,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.string.d.ts",
              "size": 1553
            },
            {
              "hash": "e4f1c05e9a49e3de695cf75fed5415fe98e54923795387489f4f1c037da76f84",
              "modified_ns": 1792957717552430978,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.weakref.d.ts",
              "size": 3132
            },
            {
              "hash": "2b64f29e43caabbe300d5dfb188ee1054726522781fc45b7b5452e45fcaad078",
              "modified_ns": 1792957717552998099,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.array.d.ts",
              "size": 4637
            },
            {
              "hash": "c9a0ba9f451cf519b049fc179f779f167fb300c850176198f700714859233697",
              "modified_ns": 1792957717553386471,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.d.ts",
              "size": 1069
            },
            {
              "hash": "a68901fc05eaf9793d9526d49e0f7d4fe4ab6207c6f8daadebbb3b41fc13f30d",
              "modified_ns": 1792957717554088007,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.error.d.ts",
              "size": 2340
            },
            {
              "hash": "5cba249fe775c2fb41836f210595a6b27e21a66bd5ecc9ae75e1fd27d4ee5432",
              "modified_ns": 1792957717554672169,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.full.d.ts",
              "size": 1035
            },
            {
              "hash": "0042a02102025c4096063681354c9985a1fde69709ae2fdd26251652572b73e9",
              "modified_ns": 1792957717555254580,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.intl.d.ts",
              "size": 7653
            },
            {
              "hash": "b8e9d9726e11e01d7d8cebcd6cc766f7a8a1b8ee8df75bea597c24ed98c7e483",
              "modified_ns": 1792957717556013824,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.object.d.ts",
              "size": 1046
            },
            {
              "hash": "f275ddf1296cce179b17fea1cabfa24e07a471f48c64f05cc2a33cb77c81814b",
              "modified_ns": 1792957717556423071,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.regexp.d.ts",
              "size": 1304
            },
            {
              "hash": "e48735ef4ea80a0480a8be5615bd55c7038be54c91fc8446ef860a6f264009e4",
              "modified_ns": 1792957717556995900,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.string.d.ts",
              "size": 1116
            },
            {
              "hash": "1cc2939a6708eeb8151456d3e951da0e87808a377e3c276e7f7b400312d4748c",
              "modified_ns": 1792957717572383567,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es5.d.ts",
              "size": 218972
            },
            {
              "hash": "56782d3cd48a39d61c88a604d37acbcb554b919e70902efb2e58b6a3d6a53fdc",
              "modified_ns": 1792957717579633383,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.scripthost.d.ts",
              "size": 9412
            },
            {
              "hash": "9c2b90984dff04a5c65c1a5a7b1c6015995902f4d181121022e3c93c5c14569c",
              "modified_ns": 1792957717589704634,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.webworker.importscripts.d.ts",
              "size": 1002
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/package.json",
              "size": 0
            },
            {
              "hash": "2437e3c47b66a7f5a4a475a16602d5bb2c28d466e2b1903624d9bbed5e383948",
              "modified_ns": 1792957717390966626,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/typescript.js",
              "size": 9144216
            },
            {
              "hash": "a4f4d978fbcbdd253a32541f8046adba91ce9b6fd98b4d5a59d501a3a653f045",
              "modified_ns": 1792957717479383823,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/package.json",
              "size": 3527
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/.pnp.cjs",
              "size": 0
            },
            {
              "excluded_children": [
                ".next",
                "coverage",
                "dist"
              ],
              "hash": "4a69b7bfe3dd36def0ca31c9b807e24cf9c93151990f179fdf7fd7fac885962c",
              "modified_ns": 0,
              "path": "/private<temporary-project>",
              "size": 0
            },
            {
              "hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
              "modified_ns": 0,
              "path": "/private<temporary-project>/.codecallnavigator",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "c4ebdf9edda4ffd4ca5ffc70905eddcfd1ebabea020d6647afd59bfbaa349bfa",
              "modified_ns": 1794509284564080813,
              "path": "/private<temporary-project>/entry.ts",
              "size": 171
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/middleware.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/middleware.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.mjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "0ac3b35f697207bd3d3ea5fbe730a6b0bbd2deeba61cda3efccb63c053384174",
              "modified_ns": 1794509284564179647,
              "path": "/private<temporary-project>/other.ts",
              "size": 46
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/proxy.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/proxy.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/middleware.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/middleware.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/proxy.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/proxy.ts",
              "size": 0
            },
            {
              "hash": "fc5af23b21e6933d7de97c23b8d21ef90e7834c0204830785b26bdcfc55e6438",
              "modified_ns": 1794509284563989521,
              "path": "/private<temporary-project>/tsconfig.json",
              "size": 83
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/.codecallnavigator/boundaries.yml",
              "size": 0
            }
          ],
          "metrics": {
            "complete": true,
            "counters": {
              "fingerprint.bytes_hashed": 59925985,
              "fingerprint.files_hashed": 350,
              "inference.cache_hits": 0,
              "inference.cache_misses": 0,
              "inference.cache_rejected": 0,
              "relationships": 1,
              "subprocesses": 1,
              "symbols": 6,
              "worker_processes": 1
            },
            "elapsed_milliseconds": 636.578292,
            "schema": "codecallnavigator.performance.v1",
            "stages": {
              "analysis": {
                "calls": 1,
                "milliseconds": 617.8225000000001
              },
              "composition": {
                "calls": 1,
                "milliseconds": 0.016708
              },
              "fingerprints.capture": {
                "calls": 2,
                "milliseconds": 7.6095
              },
              "fingerprints.verify": {
                "calls": 5,
                "milliseconds": 28.93925
              },
              "inference.boundaries": {
                "calls": 1,
                "milliseconds": 0.080125
              },
              "inference.cache_commit": {
                "calls": 1,
                "milliseconds": 1.267334
              },
              "preparation": {
                "calls": 1,
                "milliseconds": 295.077583
              },
              "typescript.extract": {
                "calls": 1,
                "milliseconds": 307.98537500000003
              },
              "typescript.prepare": {
                "calls": 1,
                "milliseconds": 295.025916
              },
              "verification": {
                "calls": 3,
                "milliseconds": 21.575125
              }
            }
          },
          "roots": [
            [
              "repo",
              "/private<temporary-project>"
            ]
          ]
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "graph_status",
    "arguments": {},
    "milliseconds": 4.32,
    "bytes": 657,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": 1,
          "nodes": 6,
          "omitted_diagnostics": 0,
          "shortened_diagnostics": 0
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "find_symbols",
    "arguments": {
      "text": "start",
      "category": "methods",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.48,
    "bytes": 1415,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "hits": [
            {
              "detail": "(flag: boolean): number \u00b7 agent-graph-io2met80 \u00b7 /private<temporary-project>/tsconfig.json",
              "name": "start",
              "target": {
                "kind": "method",
                "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
              }
            },
            {
              "detail": "(): number \u00b7 agent-graph-io2met80 \u00b7 /private<temporary-project>/tsconfig.json",
              "name": "start",
              "target": {
                "kind": "method",
                "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/other.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(): number\\\\\\\"]\\\",0]\"]"
              }
            }
          ],
          "limit": 50,
          "more": false,
          "offset": 0
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "symbol_details",
    "arguments": {
      "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.35,
    "bytes": 1209,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "body_unavailable": false,
          "build": "/private<temporary-project>/tsconfig.json",
          "container": "entry.ts",
          "file": "/private<temporary-project>/entry.ts",
          "id": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
          "is_generated": false,
          "is_test": false,
          "kind": "function",
          "label": "start",
          "language": "typescript",
          "line": 2,
          "namespace": ".",
          "project": "agent-graph-io2met80",
          "signature": "(flag: boolean): number"
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "function_flow",
    "arguments": {
      "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.26,
    "bytes": 4367,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "chart": {
            "diagnostics": [],
            "entry": 0,
            "file": "/private<temporary-project>/entry.ts",
            "links": [
              {
                "from": 0,
                "kind": "normal",
                "label": "",
                "to": 1
              },
              {
                "from": 1,
                "kind": "yes",
                "label": "true",
                "to": 2
              },
              {
                "from": 2,
                "kind": "normal",
                "label": "",
                "to": 3
              },
              {
                "from": 1,
                "kind": "no",
                "label": "false",
                "to": 4
              },
              {
                "from": 4,
                "kind": "normal",
                "label": "",
                "to": 5
              },
              {
                "from": 3,
                "kind": "normal",
                "label": "",
                "to": 6
              },
              {
                "from": 5,
                "kind": "normal",
                "label": "",
                "to": 6
              }
            ],
            "partial": false,
            "regions": [],
            "span": {
              "end": 127,
              "start": 45
            },
            "steps": [
              {
                "calls": [],
                "id": 0,
                "kind": "start",
                "label": "Function entry",
                "span": {
                  "end": 127,
                  "start": 45
                }
              },
              {
                "calls": [],
                "id": 1,
                "kind": "decision",
                "label": "flag",
                "span": {
                  "end": 99,
                  "start": 95
                }
              },
              {
                "calls": [
                  {
                    "diagnostic": null,
                    "label": "leaf",
                    "span": {
                      "end": 114,
                      "start": 108
                    },
                    "targets": [
                      {
                        "certainty": "direct",
                        "provenance": "typescript.compiler",
                        "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"leaf\\\\\\\",\\\\\\\"(): number\\\\\\\"]\\\",0]\"]"
                      }
                    ]
                  }
                ],
                "id": 2,
                "kind": "expression",
                "label": "leaf()",
                "span": {
                  "end": 114,
                  "start": 108
                }
              },
              {
                "calls": [],
                "id": 3,
                "kind": "return",
                "label": "return",
                "span": {
                  "end": 115,
                  "start": 101
                }
              },
              {
                "calls": [],
                "id": 4,
                "kind": "expression",
                "label": "0",
                "span": {
                  "end": 124,
                  "start": 123
                }
              },
              {
                "calls": [],
                "id": 5,
                "kind": "return",
                "label": "return",
                "span": {
                  "end": 125,
                  "start": 116
                }
              },
              {
                "calls": [],
                "id": 6,
                "kind": "exit",
                "label": "Return to caller",
                "span": {
                  "end": 127,
                  "start": 45
                }
              }
            ],
            "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
          },
          "function": {
            "body": [
              {
                "condition": {
                  "calls": [],
                  "kind": "value",
                  "regions": [],
                  "span": {
                    "end": 99,
                    "start": 95
                  },
                  "text": "flag"
                },
                "else_branch": [],
                "kind": "if",
                "span": {
                  "end": 115,
                  "start": 92
                },
                "then_branch": [
                  {
                    "kind": "return",
                    "span": {
                      "end": 115,
                      "start": 101
                    },
                    "value": {
                      "calls": [
                        {
                          "diagnostic": null,
                          "label": "leaf",
                          "span": {
                            "end": 114,
                            "start": 108
                          },
                          "targets": [
                            {
                              "certainty": "direct",
                              "provenance": "typescript.compiler",
                              "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"leaf\\\\\\\",\\\\\\\"(): number\\\\\\\"]\\\",0]\"]"
                            }
                          ]
                        }
                      ],
                      "kind": "value",
                      "regions": [],
                      "span": {
                        "end": 114,
                        "start": 108
                      },
                      "text": "leaf()"
                    }
                  }
                ]
              },
              {
                "kind": "return",
                "span": {
                  "end": 125,
                  "start": 116
                },
                "value": {
                  "calls": [],
                  "kind": "value",
                  "regions": [],
                  "span": {
                    "end": 124,
                    "start": 123
                  },
                  "text": "0"
                }
              }
            ],
            "build": "/private<temporary-project>/tsconfig.json",
            "diagnostics": [],
            "file": "/private<temporary-project>/entry.ts",
            "language": "typescript",
            "project": "agent-graph-io2met80",
            "span": {
              "end": 127,
              "start": 45
            },
            "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
          },
          "source": {
            "content": "export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\nexport interface Contract { run(): void; }\n",
            "file": "/private<temporary-project>/entry.ts",
            "line": 2,
            "origin": "analyzed",
            "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
          }
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "neighbors",
    "arguments": {
      "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
      "limits": {
        "nodes": 1,
        "edges": 1
      },
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.46,
    "bytes": 1606,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": [],
          "nodes": [
            {
              "body_unavailable": false,
              "build": "/private<temporary-project>/tsconfig.json",
              "container": "entry.ts",
              "file": "/private<temporary-project>/entry.ts",
              "id": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
              "is_generated": false,
              "is_test": false,
              "kind": "function",
              "label": "start",
              "language": "typescript",
              "line": 2,
              "namespace": ".",
              "project": "agent-graph-io2met80",
              "signature": "(flag: boolean): number"
            }
          ],
          "omitted_diagnostics": 0,
          "omitted_edges": 1,
          "omitted_nodes": 1,
          "roots": [
            "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
          ],
          "shortened_diagnostics": 0,
          "truncated": true
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "graph_section",
    "arguments": {
      "selection": {
        "kind": "neighborhood",
        "symbols": [
          "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
        ],
        "direction": "Both",
        "depth": 2
      },
      "limits": {
        "nodes": 1,
        "edges": 1
      },
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.2,
    "bytes": 1606,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": [],
          "nodes": [
            {
              "body_unavailable": false,
              "build": "/private<temporary-project>/tsconfig.json",
              "container": "entry.ts",
              "file": "/private<temporary-project>/entry.ts",
              "id": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
              "is_generated": false,
              "is_test": false,
              "kind": "function",
              "label": "start",
              "language": "typescript",
              "line": 2,
              "namespace": ".",
              "project": "agent-graph-io2met80",
              "signature": "(flag: boolean): number"
            }
          ],
          "omitted_diagnostics": 0,
          "omitted_edges": 1,
          "omitted_nodes": 1,
          "roots": [
            "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
          ],
          "shortened_diagnostics": 0,
          "truncated": true
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "find_symbols",
    "arguments": {
      "text": "run",
      "category": "methods",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.1,
    "bytes": 1301,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "hits": [
            {
              "detail": "(): void \u00b7 agent-graph-io2met80 \u00b7 /private<temporary-project>/tsconfig.json",
              "name": "run",
              "target": {
                "kind": "method",
                "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"[\\\\\\\\\\\\\\\"[\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"<module>\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"InterfaceDeclaration\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"Contract\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"]\\\\\\\\\\\\\\\",0]\\\\\\\",\\\\\\\"MethodSignature\\\\\\\",\\\\\\\"run\\\\\\\",\\\\\\\"(): void\\\\\\\"]\\\",0]\"]"
              }
            }
          ],
          "limit": 50,
          "more": false,
          "offset": 0
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "function_flow",
    "arguments": {
      "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"[\\\\\\\\\\\\\\\"[\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"<module>\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"InterfaceDeclaration\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"Contract\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\"]\\\\\\\\\\\\\\\",0]\\\\\\\",\\\\\\\"MethodSignature\\\\\\\",\\\\\\\"run\\\\\\\",\\\\\\\"(): void\\\\\\\"]\\\",0]\"]",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.08,
    "bytes": 621,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI error flow_unavailable: No source body is available for this declaration."
        }
      ],
      "structuredContent": {
        "error": {
          "code": "flow_unavailable",
          "message": "No source body is available for this declaration."
        },
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "unverifiable",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "reasons_total": 1,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": null,
        "version": 1
      },
      "isError": true
    }
  },
  {
    "tool": "graph_status",
    "arguments": {},
    "milliseconds": 3.02,
    "bytes": 703,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "stale",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            },
            {
              "code": "input_changed",
              "path": "@repo/other.ts"
            }
          ],
          "reasons_total": 2,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": 1,
          "nodes": 6,
          "omitted_diagnostics": 0,
          "shortened_diagnostics": 0
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "symbol_source",
    "arguments": {
      "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]",
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 3.03,
    "bytes": 1116,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "stale",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            },
            {
              "code": "input_changed",
              "path": "@repo/other.ts"
            }
          ],
          "reasons_total": 2,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "content": "export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\nexport interface Contract { run(): void; }\n",
          "file": "/private<temporary-project>/entry.ts",
          "line": 2,
          "origin": "analyzed",
          "symbol": "[\"typescript\",\"@repo/tsconfig.json\",\"@repo/entry.ts\",\"[\\\"[\\\\\\\"<module>\\\\\\\",\\\\\\\"FunctionDeclaration\\\\\\\",\\\\\\\"start\\\\\\\",\\\\\\\"(flag: boolean): number\\\\\\\"]\\\",0]\"]"
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "graph_status",
    "arguments": {
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 2.98,
    "bytes": 703,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": {
          "contract_valid": true,
          "digest": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
          "input_status": "stale",
          "omitted_reasons": 0,
          "reasons": [
            {
              "code": "unmapped_input"
            },
            {
              "code": "input_changed",
              "path": "@repo/other.ts"
            }
          ],
          "reasons_total": 2,
          "version": 1
        },
        "generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932",
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": 1,
          "nodes": 6,
          "omitted_diagnostics": 0,
          "shortened_diagnostics": 0
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "refresh_graph",
    "arguments": {},
    "milliseconds": 646.97,
    "bytes": 29347,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI operation completed; structuredContent contains the full response and metadata."
        }
      ],
      "structuredContent": {
        "error": null,
        "freshness": null,
        "generation": "805422027b40b3790e0037b996de394381ef300ec438f2c51f6fc01460c898c5",
        "result": {
          "digest": "805422027b40b3790e0037b996de394381ef300ec438f2c51f6fc01460c898c5",
          "engine_versions": "{\"callatlas\":{\"legacy_patch\":\"patches/callatlas-assembly-identity-legacy.patch\",\"legacy_patch_sha256\":\"ec723795ee841b98c2efbf49a7f010cd3f22652889c0e5c34866461b0137c574\",\"older_patch\":\"patches/callatlas-assembly-identity-entries.patch\",\"older_patch_sha256\":\"aeb8b8b664e21331e7815466746ec482a63407e4ae05ce8275e2810f552ff405\",\"patch\":\"patches/callatlas-assembly-identity.patch\",\"patch_sha256\":\"7faebb71f4a998f62bac10bd44741169bfd412ad0f006fbd4698d8d8c58b562b\",\"pre_framework_patch\":\"patches/callatlas-assembly-identity-pre-framework.patch\",\"pre_framework_patch_sha256\":\"7b64792f14f02fafb4472402b0ab26ac53d9be6e496ef73964ad49791db1d08a\",\"pre_http_patch\":\"patches/callatlas-assembly-identity-pre-http.patch\",\"pre_http_patch_sha256\":\"3dc7ede680f323e7e394ecdd5ef9551f90ca1cf5413bed589241b47a7834c216\",\"pre_review_patch\":\"patches/callatlas-assembly-identity-pre-review.patch\",\"pre_review_patch_sha256\":\"874c4dbd8543abb34e6c87b634e4cf3d67acc300b80a66d28e58fbaafe90152e\",\"pre_visibility_patch\":\"patches/callatlas-assembly-identity-pre-visibility.patch\",\"pre_visibility_patch_sha256\":\"c9633858854be41ab7e6867df8727a74de6674363f20636216555ed7cfe7472c\",\"previous_patch\":\"patches/callatlas-assembly-identity-policy.patch\",\"previous_patch_sha256\":\"a3981a0c07ef932a498f58113994c7858bd53242e0f0860355af9df56704e810\",\"repository\":\"https://github.com/castorhrio/callatlas.git\",\"revision\":\"916c2419c0cdb91b5b86b84df4842dadb17bb6b4\"},\"callatlas_framework_facts\":\"1\",\"callatlas_http_facts\":\"1\",\"callatlas_protocol\":\"callatlas.ir.v3\",\"java-callgraph2\":{\"bcel_version\":\"6.13.0\",\"repository\":\"https://github.com/Adrninistrator/java-callgraph2.git\",\"revision\":\"77f566d0878958bab929b4cdeafcc2c762ff555e\",\"version\":\"4.1.0\"},\"projects\":{\"/private<temporary-project>/tsconfig.json\":{\"adapter\":\"6\",\"boundary_rules\":\"1\",\"next:/private<temporary-project>/tsconfig.json\":\"16.4.2\",\"next_rules\":\"1\",\"node\":\"24.15.0\",\"react:/private<temporary-project>/tsconfig.json\":\"19.3.1\",\"react_rules\":\"1\",\"typescript\":\"6.0.3\",\"typescript:/private<temporary-project>/tsconfig.json\":\"6.0.3\"}}}",
          "event": "complete",
          "fingerprints": [
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "0947112b8571fa3276f658586d7c8c50e40b258dc05f848b6d0e166903307ebb",
              "modified_ns": 1792957718748976406,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/next/package.json",
              "size": 3188
            },
            {
              "hash": "4974b7aaa9319a549185409d032bc92427826bc7c4804eed8b24be54d1995c3e",
              "modified_ns": 1792957716891906647,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/react/package.json",
              "size": 1242
            },
            {
              "hash": "dee990117dfbc2b88b7eb05253bfd7fe6b33e60cb1cd4a3c85d80ba5cbe5f031",
              "modified_ns": 1792957717482301508,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.decorators.d.ts",
              "size": 13153
            },
            {
              "hash": "65e3a677b1f0808cc6c676010cee5449abdaf4535997528eb59631ecc42da55d",
              "modified_ns": 1792957717483231625,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.decorators.legacy.d.ts",
              "size": 1287
            },
            {
              "hash": "ad664232e56da1d48c64483a79fe96d704f4459516f80c6d71b6d281541d33bd",
              "modified_ns": 1792957717483840245,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.asynciterable.d.ts",
              "size": 933
            },
            {
              "hash": "6147499cf009f666b611af183db7812800a24369b4c7c27ddd367860bc730790",
              "modified_ns": 1792957717509715785,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.d.ts",
              "size": 2349483
            },
            {
              "hash": "ad664232e56da1d48c64483a79fe96d704f4459516f80c6d71b6d281541d33bd",
              "modified_ns": 1792957717510443237,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.dom.iterable.d.ts",
              "size": 933
            },
            {
              "hash": "8110e907b0282869297b46b8f66980b24bbfbef194ebf04008026c8dc910b21b",
              "modified_ns": 1792957717510913150,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.collection.d.ts",
              "size": 5305
            },
            {
              "hash": "dcd33183cd1eceac1796fe643a1804ed7c4537947488911f433f13be1be64315",
              "modified_ns": 1792957717512083057,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.core.d.ts",
              "size": 22824
            },
            {
              "hash": "ebb9c892e0747f62a3d9e5203876c36f506d89933d6c0134540a7819112a45c9",
              "modified_ns": 1792957717512809301,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.d.ts",
              "size": 1202
            },
            {
              "hash": "e314f64d0d8478d702e429ff2d89dff8737e65a3f4509a3757791eea8c5c7e99",
              "modified_ns": 1792957717513334880,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.generator.d.ts",
              "size": 2518
            },
            {
              "hash": "592ac08b76799d1cfeb707171a88f79fa2f3b51a5c9c2fa65b6d8db9fd5bdb61",
              "modified_ns": 1792957717514120582,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.iterable.d.ts",
              "size": 18185
            },
            {
              "hash": "cc23ff12135871b0b3409a2042d3d9ccecaa3c111e0f73c81eed990e9752d362",
              "modified_ns": 1792957717514852159,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.promise.d.ts",
              "size": 3161
            },
            {
              "hash": "a32ee6a6af28daa75edcd23b25c3231162ac23887baf9a91fa184656acba4908",
              "modified_ns": 1792957717515637945,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.proxy.d.ts",
              "size": 5212
            },
            {
              "hash": "4433220e9ff2107fa4fe1bcaa88ef1682a98669d663a66cf37adbcebb958f67f",
              "modified_ns": 1792957717516614437,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.reflect.d.ts",
              "size": 6454
            },
            {
              "hash": "4ea16d68d5276616ccdec1b716993a8b3bb8207eee025c8f2fbd8d370207eec4",
              "modified_ns": 1792957717516932767,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.symbol.d.ts",
              "size": 1610
            },
            {
              "hash": "987b64e5d8cebc263aa7f1b2de2eb3a64373a61bd444ce73cfd092434408d263",
              "modified_ns": 1792957717517568804,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2015.symbol.wellknown.d.ts",
              "size": 10971
            },
            {
              "hash": "0ad35396159f585928341f576a1b46aa170a439cfe49278e16b565c792b56bbc",
              "modified_ns": 1792957717525173284,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.array.include.d.ts",
              "size": 5165
            },
            {
              "hash": "75aaeb4453119518c1099ac5c05c76d433cb83ad19a4c89a6f47d89ed2179260",
              "modified_ns": 1792957717525511364,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.d.ts",
              "size": 926
            },
            {
              "hash": "a9b114a2761723978c531b66d27a5285f08c01fe3e538ae0b456c578cf996f81",
              "modified_ns": 1792957717526596730,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2016.intl.d.ts",
              "size": 1445
            },
            {
              "hash": "89b2d8fe38506fe46cf087c1640126c192b543d59b59644284c3e9240061445e",
              "modified_ns": 1792957717527379307,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.arraybuffer.d.ts",
              "size": 876
            },
            {
              "hash": "851df1d49109faccb894aedec02d5672b42ade3a0fc03c64b7d67c005fb1a670",
              "modified_ns": 1792957717528089802,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.d.ts",
              "size": 1123
            },
            {
              "hash": "40322fc8016bf926f02a930394867f9ae5a74c35f619602568b771a667189ce7",
              "modified_ns": 1792957717528540340,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.date.d.ts",
              "size": 1877
            },
            {
              "hash": "2e77da3823b36098b6ea9f602e4ba0fa03079f5ada9adb2d4dddfe8591e5f443",
              "modified_ns": 1792957717529845746,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.intl.d.ts",
              "size": 1410
            },
            {
              "hash": "8171c1c57b999098bf2b02442a374df2ab108bde7e9f3ded2d3a0779896bb5fb",
              "modified_ns": 1792957717530487740,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.object.d.ts",
              "size": 2434
            },
            {
              "hash": "7f85b7c8b57ca90da2d85e2e52f4e264371f1744d17fee15d413372cf0351f32",
              "modified_ns": 1792957717531288317,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.sharedmemory.d.ts",
              "size": 7170
            },
            {
              "hash": "e0cccda8bc4a5bfd01f7d1c9f007b2251cd768b97335ac3b15a1e3a0f3862ff7",
              "modified_ns": 1792957717532081644,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.string.d.ts",
              "size": 2339
            },
            {
              "hash": "1bb8797ed5460005e848ce6153a49ba0d6d2e9a0985cd3d153752a8713d09df4",
              "modified_ns": 1792957717532369392,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2017.typedarrays.d.ts",
              "size": 1503
            },
            {
              "hash": "761a0763ae83dae69cb358eddf073bd3f01edbdc467510851e62b2d3ba0fc6b1",
              "modified_ns": 1792957717533056511,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.asyncgenerator.d.ts",
              "size": 2653
            },
            {
              "hash": "5036834ea5402e07381d56ce8d1d58263662380e10102fc832e67fae9236ede9",
              "modified_ns": 1792957717533762130,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.asynciterable.d.ts",
              "size": 2218
            },
            {
              "hash": "1c2eb86a62014e4418b640799b9e85da46c95c0f4ff0a1fec7fe1b77fda53771",
              "modified_ns": 1792957717535207577,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.d.ts",
              "size": 1049
            },
            {
              "hash": "6ca970975088545700c7dff03c96a4e2f0072ae320255fcbe6f9afc09fcfae7c",
              "modified_ns": 1792957717537052395,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.intl.d.ts",
              "size": 2982
            },
            {
              "hash": "ee19ed8ba7b26224ce45dd97f874c7a361c1370b7c812a0585460727c94fcecf",
              "modified_ns": 1792957717537502642,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.promise.d.ts",
              "size": 1314
            },
            {
              "hash": "fb4b7dd4d24b0818898cf50eb81d106c7e4dc0bb5f56f019fcc55318783cc41c",
              "modified_ns": 1792957717537905763,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2018.regexp.d.ts",
              "size": 1193
            },
            {
              "hash": "a4246a2e5319c6eaa00b5bc0400bb4be35960dc7a6260ff9b2b62d9c5a1880c6",
              "modified_ns": 1792957717538797214,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.array.d.ts",
              "size": 3124
            },
            {
              "hash": "533814bf32853a8ce284a10f674376a6645113f1280d299f947378a3bda42b01",
              "modified_ns": 1792957717539556875,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.d.ts",
              "size": 1032
            },
            {
              "hash": "add61133869c6272e0c25fad9773eb8903511d5bc44b8d13412a708266834618",
              "modified_ns": 1792957717540304410,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.intl.d.ts",
              "size": 919
            },
            {
              "hash": "4dc687f0ba123d1c25b2da24855c4f575982d8f94bfecdbb1a78cf6fbf7c9206",
              "modified_ns": 1792957717540901031,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.object.d.ts",
              "size": 1434
            },
            {
              "hash": "961f733c1224441b118521a287c9b7601fae8631ec5f4eecd18a17fbef8f27be",
              "modified_ns": 1792957717541325902,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.string.d.ts",
              "size": 1489
            },
            {
              "hash": "e5dd064c8f5e7946fefd0a3aa310ed687b42c6103f09b7469f197f4e367973ec",
              "modified_ns": 1792957717541854565,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2019.symbol.d.ts",
              "size": 967
            },
            {
              "hash": "06c85612d54ca055608b257dd12e0b6649b3aaf9da515a0bab0dc64ac222634d",
              "modified_ns": 1792957717543032013,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.bigint.d.ts",
              "size": 38258
            },
            {
              "hash": "359ae19ab0853e9c5ed05f8d7cbf1ac5274f807b26a7a6909808a0190b7fceab",
              "modified_ns": 1792957717543727883,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.d.ts",
              "size": 1162
            },
            {
              "hash": "8654796f949b33be4c464d898336329f78eea030fd2031dc5fd2ab684e5e2be8",
              "modified_ns": 1792957717544045838,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.date.d.ts",
              "size": 2917
            },
            {
              "hash": "4d2bc55af74f7e40ee234b15415f64de5dfc3b310d47d8616562686b58674f1e",
              "modified_ns": 1792957717545508535,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.intl.d.ts",
              "size": 22399
            },
            {
              "hash": "58a0aee7d9307e467886da0b2543f8484b194843bf8678d9dd819771e61c2cd3",
              "modified_ns": 1792957717546307153,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.number.d.ts",
              "size": 1548
            },
            {
              "hash": "c11c0a3ddb849e910921cf0237bf03ede59453012b9e4ff9a6106f48193df8b1",
              "modified_ns": 1792957717546746775,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.promise.d.ts",
              "size": 1764
            },
            {
              "hash": "d6b0c2a40f02259a8ccaceb25409975d2ddf675bde1cd38b0f4a17d54cdc0c17",
              "modified_ns": 1792957717547637892,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.sharedmemory.d.ts",
              "size": 5089
            },
            {
              "hash": "aba305cace33a1524511f471c385855e04ab4eba415cf05433fa6028c5461df5",
              "modified_ns": 1792957717547890432,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.string.d.ts",
              "size": 2451
            },
            {
              "hash": "8d6a36d523956e0f594f1ca3fdcadf2923fcd2b45072403ae2edf87b62bdb9b3",
              "modified_ns": 1792957717548410095,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2020.symbol.wellknown.d.ts",
              "size": 1566
            },
            {
              "hash": "6ef27e928c0977a62ce2323d0709db70be4b76c160c446095340d7e4c5d4690f",
              "modified_ns": 1792957717549072256,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.d.ts",
              "size": 997
            },
            {
              "hash": "e5b976405b0b3c8f4e85a1e371c2b91a2ac55a81b93ed34990495e80ee98ac1a",
              "modified_ns": 1792957717550261829,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.intl.d.ts",
              "size": 8206
            },
            {
              "hash": "a3a6fee8121f9153f108329375311f975638efcb83aef20dfef9b2692861fbf4",
              "modified_ns": 1792957717550795909,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.promise.d.ts",
              "size": 2223
            },
            {
              "hash": "4ab97a788f157b3d3015049d8a0b7e2c8c970316dbfbac5e4c615786dc0a93f7",
              "modified_ns": 1792957717551407195,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.string.d.ts",
              "size": 1553
            },
            {
              "hash": "e4f1c05e9a49e3de695cf75fed5415fe98e54923795387489f4f1c037da76f84",
              "modified_ns": 1792957717552430978,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2021.weakref.d.ts",
              "size": 3132
            },
            {
              "hash": "2b64f29e43caabbe300d5dfb188ee1054726522781fc45b7b5452e45fcaad078",
              "modified_ns": 1792957717552998099,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.array.d.ts",
              "size": 4637
            },
            {
              "hash": "c9a0ba9f451cf519b049fc179f779f167fb300c850176198f700714859233697",
              "modified_ns": 1792957717553386471,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.d.ts",
              "size": 1069
            },
            {
              "hash": "a68901fc05eaf9793d9526d49e0f7d4fe4ab6207c6f8daadebbb3b41fc13f30d",
              "modified_ns": 1792957717554088007,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.error.d.ts",
              "size": 2340
            },
            {
              "hash": "5cba249fe775c2fb41836f210595a6b27e21a66bd5ecc9ae75e1fd27d4ee5432",
              "modified_ns": 1792957717554672169,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.full.d.ts",
              "size": 1035
            },
            {
              "hash": "0042a02102025c4096063681354c9985a1fde69709ae2fdd26251652572b73e9",
              "modified_ns": 1792957717555254580,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.intl.d.ts",
              "size": 7653
            },
            {
              "hash": "b8e9d9726e11e01d7d8cebcd6cc766f7a8a1b8ee8df75bea597c24ed98c7e483",
              "modified_ns": 1792957717556013824,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.object.d.ts",
              "size": 1046
            },
            {
              "hash": "f275ddf1296cce179b17fea1cabfa24e07a471f48c64f05cc2a33cb77c81814b",
              "modified_ns": 1792957717556423071,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.regexp.d.ts",
              "size": 1304
            },
            {
              "hash": "e48735ef4ea80a0480a8be5615bd55c7038be54c91fc8446ef860a6f264009e4",
              "modified_ns": 1792957717556995900,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es2022.string.d.ts",
              "size": 1116
            },
            {
              "hash": "1cc2939a6708eeb8151456d3e951da0e87808a377e3c276e7f7b400312d4748c",
              "modified_ns": 1792957717572383567,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.es5.d.ts",
              "size": 218972
            },
            {
              "hash": "56782d3cd48a39d61c88a604d37acbcb554b919e70902efb2e58b6a3d6a53fdc",
              "modified_ns": 1792957717579633383,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.scripthost.d.ts",
              "size": 9412
            },
            {
              "hash": "9c2b90984dff04a5c65c1a5a7b1c6015995902f4d181121022e3c93c5c14569c",
              "modified_ns": 1792957717589704634,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/lib.webworker.importscripts.d.ts",
              "size": 1002
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/package.json",
              "size": 0
            },
            {
              "hash": "2437e3c47b66a7f5a4a475a16602d5bb2c28d466e2b1903624d9bbed5e383948",
              "modified_ns": 1792957717390966626,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/lib/typescript.js",
              "size": 9144216
            },
            {
              "hash": "a4f4d978fbcbdd253a32541f8046adba91ce9b6fd98b4d5a59d501a3a653f045",
              "modified_ns": 1792957717479383823,
              "path": "/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript/package.json",
              "size": 3527
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/.pnp.cjs",
              "size": 0
            },
            {
              "excluded_children": [
                ".next",
                "coverage",
                "dist"
              ],
              "hash": "4a69b7bfe3dd36def0ca31c9b807e24cf9c93151990f179fdf7fd7fac885962c",
              "modified_ns": 0,
              "path": "/private<temporary-project>",
              "size": 0
            },
            {
              "hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
              "modified_ns": 0,
              "path": "/private<temporary-project>/.codecallnavigator",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/.pnp.cjs",
              "size": 0
            },
            {
              "hash": "c4ebdf9edda4ffd4ca5ffc70905eddcfd1ebabea020d6647afd59bfbaa349bfa",
              "modified_ns": 1794509284564080813,
              "path": "/private<temporary-project>/entry.ts",
              "size": 171
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/middleware.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/middleware.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.mjs",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/next.config.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "49fb3809f4b48db0db233a76283612fc428f36fde41b292cda6ea881049f63e7",
              "modified_ns": 1794509285254135259,
              "path": "/private<temporary-project>/other.ts",
              "size": 47
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/proxy.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/proxy.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/middleware.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/middleware.ts",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/proxy.js",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/src/proxy.ts",
              "size": 0
            },
            {
              "hash": "fc5af23b21e6933d7de97c23b8d21ef90e7834c0204830785b26bdcfc55e6438",
              "modified_ns": 1794509284563989521,
              "path": "/private<temporary-project>/tsconfig.json",
              "size": 83
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/T/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/j3qnf2cn5fx1tqgq8gm50vf80000gq/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/9t/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/folders/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/npm-shrinkwrap.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/package-lock.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/package.json",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/pnpm-lock.yaml",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/var/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/yarn.lock",
              "size": 0
            },
            {
              "hash": "",
              "missing": true,
              "modified_ns": 0,
              "path": "/private<temporary-project>/.codecallnavigator/boundaries.yml",
              "size": 0
            }
          ],
          "metrics": {
            "complete": true,
            "counters": {
              "fingerprint.bytes_hashed": 59925990,
              "fingerprint.files_hashed": 350,
              "inference.cache_hits": 0,
              "inference.cache_misses": 0,
              "inference.cache_rejected": 0,
              "relationships": 1,
              "subprocesses": 1,
              "symbols": 6,
              "worker_processes": 1
            },
            "elapsed_milliseconds": 640.972459,
            "schema": "codecallnavigator.performance.v1",
            "stages": {
              "analysis": {
                "calls": 1,
                "milliseconds": 622.4502090000001
              },
              "composition": {
                "calls": 1,
                "milliseconds": 0.014625
              },
              "fingerprints.capture": {
                "calls": 2,
                "milliseconds": 7.384167000000001
              },
              "fingerprints.verify": {
                "calls": 5,
                "milliseconds": 28.227041999999997
              },
              "inference.boundaries": {
                "calls": 1,
                "milliseconds": 0.390375
              },
              "inference.cache_commit": {
                "calls": 1,
                "milliseconds": 0.817875
              },
              "preparation": {
                "calls": 1,
                "milliseconds": 303.80975
              },
              "typescript.extract": {
                "calls": 1,
                "milliseconds": 304.747292
              },
              "typescript.prepare": {
                "calls": 1,
                "milliseconds": 303.75475
              },
              "verification": {
                "calls": 3,
                "milliseconds": 20.934626
              }
            }
          },
          "roots": [
            [
              "repo",
              "/private<temporary-project>"
            ]
          ]
        },
        "version": 1
      },
      "isError": false
    }
  },
  {
    "tool": "graph_status",
    "arguments": {
      "expected_generation": "491064a6beb42e28ad3fa5a9bd9e9ac0631436a266c4275ad2d18e7638ddc932"
    },
    "milliseconds": 4.48,
    "bytes": 352,
    "response": {
      "content": [
        {
          "type": "text",
          "text": "CLI error generation_mismatch: The saved graph changed; reload before combining results"
        }
      ],
      "structuredContent": {
        "error": {
          "code": "generation_mismatch",
          "message": "The saved graph changed; reload before combining results"
        },
        "freshness": null,
        "generation": null,
        "result": null,
        "version": 1
      },
      "isError": true
    }
  },
  {
    "scenario": "unavailable-engine",
    "exit": 1,
    "stderr": "Error: Cannot start configured CLI\n\nCaused by:\n    No such file or directory (os error 2)\n"
  }
]
```
