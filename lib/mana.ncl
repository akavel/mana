let map_nonempty_field = fun field_name f record =>
  if std.record.has_field field_name record
    && record."%{field_name}" != ""
  then
    record |> std.record.update field_name (f record."%{field_name}")
  else
    record
  in

let set_field_default = fun field_name default_value record =>
  if std.record.has_field field_name record
  then record
  else record |> std.record.insert field_name default_value
  in

# NOTE: expects `tree` to only have string leaves.
# TODO: annotate properly with types/contracts
let windows' = fun shadow_dir handlers tree =>
  m%"
  com.akavel.mana.v1
  shadow %{shadow_dir}
  %{
    handlers
    |> std.record.to_array
    |> std.array.map (fun h =>
        "handle %{h.field} %{h.value}"
      )
    |> std.string.join "\n"
  }
  %{
    tree
    |> (import "leafs.ncl")
    |> std.array.map (fun x => {
        path = x |> std.array.drop_last |> std.string.join "/",
        contents = x |> std.array.last,
      })
    |> std.array.map (fun x =>
        x |> map_nonempty_field "contents" (fun c =>
          "\n %{c |> std.string.replace "\n" "\n "}")
        |> set_field_default "contents" ""
      )
    |> std.array.map (fun x =>
        # TODO: urlencode path
        "want %{x.path}%{x.contents}")
    |> std.string.join "\n"
  }
  affect
  "%
  in

{
  windows = windows',
}
