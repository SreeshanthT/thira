window.BENCHMARK_DATA = {
  "lastUpdate": 1793759458398,
  "repoUrl": "https://github.com/SreeshanthT/thira",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "iam.sreeshanth.t@gmail.com",
            "name": "SreeshanthT",
            "username": "iamsreeshanth"
          },
          "committer": {
            "email": "81209050+SreeshanthT@users.noreply.github.com",
            "name": "Sreeshanth Thekkedath",
            "username": "SreeshanthT"
          },
          "distinct": true,
          "id": "fadf8abf4d4ffbbb1f5af95239352913790fe93a",
          "message": "Add benchmarks that fail PRs on regressions over 10%\n\nCriterion benchmarks cover timecode conversion, an edit and undo on a\n1,000-clip track, saving and loading a 1,000-clip project, and rendering\na 1080p frame.\n\nOn pull requests one job measures the base and head alternately for\nthree rounds and scripts/bench-compare.mjs compares each side's fastest\nround. Comparing two sequential runs does not work: identical code\nmeasured twice differed by up to 36% locally and reported three false\nregressions, while the fastest of alternating rounds stayed within 7.3%.\nA regression must be over 10% with non-overlapping confidence intervals.\n\nResults from main go to the gh-pages branch as history. The check is not\nrequired yet, until it proves it does not fail by chance.\n\nCloses #18.\n\nSigned-off-by: SreeshanthT <iam.sreeshanth.t@gmail.com>",
          "timestamp": "2026-11-04T07:57:02+05:30",
          "tree_id": "32462ea75a2e3e3048655a034076ad849603f30f",
          "url": "https://github.com/SreeshanthT/thira/commit/fadf8abf4d4ffbbb1f5af95239352913790fe93a"
        },
        "date": 1793759457540,
        "tool": "cargo",
        "benches": [
          {
            "name": "edit/insert_and_undo_on_1000_clips",
            "value": 193621,
            "range": "± 7080",
            "unit": "ns/iter"
          },
          {
            "name": "render/test_pattern_1080p",
            "value": 6319110,
            "range": "± 653802",
            "unit": "ns/iter"
          },
          {
            "name": "store/save_1000_clips",
            "value": 5193964,
            "range": "± 359602",
            "unit": "ns/iter"
          },
          {
            "name": "store/load_1000_clips",
            "value": 814415,
            "range": "± 30960",
            "unit": "ns/iter"
          },
          {
            "name": "timecode/from_frame_drop_frame",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "timecode/parse_drop_frame",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}