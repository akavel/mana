# https://github.com/tweag/nickel/discussions/1763

let leafs
  | doc m%"
      Returns all the leafs of a record as an array of paths.

      Each element of the returned outermost array, a path, is composed of the
      consective fields that need to be traversed to reach the leaf concatened
      with the value of the leaf.

      # Examples

      ```nickel
      {
        foo = {
          bar = 1,
          baz = 2,
        },
        foobis.barbis.borg = ["hello"],
      }
      |> leafs
      ```

      gives

      ```nickel
      [
        [ "foo", "bar", 1 ],
        [ "foo", "baz", 2 ],
        [ "foobis", "barbis", "borg", [ "hello" ] ]
      ]
      ```
    "%
  | { .. } -> Array (Array Dyn)
  =
    let rec leafs_aux 
      | doc m%"
        Auxiliary variant of `leafs`.

        `leafs_aux` takes a current field path, indicating the position of the
        field being processed in the original record (represented as an array of
        strings) and the value of the field (which might or might not be a
        record). Returns the set of leafs of this field, represented as an array
        of paths.
      "%
      | Array String -> Dyn -> Array (Array Dyn) = fun path value =>
        if !(std.is_record value) then
          # If we reached a leaf, we just append the value to the current path,
          # and return an array with a single element
          [std.array.append value path]
        else
          # If the value is itself a record, we simply recurse into each field,
          # generate an array of leafs for each such field.
          value
          |> std.record.to_array
          |> std.array.map
            (
              fun { field, value } =>
                let new_path = std.array.append field path in
                leafs_aux new_path value
            )
          # Now we have an array of array of fields (each subfield might have
          # several leafs itself), so we finally flatten it
          |> std.array.flatten
      in

    leafs_aux []
  in

leafs
