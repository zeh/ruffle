//! Tests running SWFs in a headless Ruffle instance.
//!
//! Trace output can be compared with correct output from the official Flash Payer.

use approx::assert_relative_eq;
use ruffle_core::backend::locale::NullLocaleBackend;
use ruffle_core::backend::log::LogBackend;
use ruffle_core::backend::navigator::{NullExecutor, NullNavigatorBackend};
use ruffle_core::backend::storage::MemoryStorageBackend;
use ruffle_core::backend::{
    audio::NullAudioBackend, input::NullInputBackend, render::NullRenderer,
};
use ruffle_core::context::UpdateContext;
use ruffle_core::external::Value as ExternalValue;
use ruffle_core::external::{ExternalInterfaceMethod, ExternalInterfaceProvider};
use ruffle_core::tag_utils::SwfMovie;
use ruffle_core::Player;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Error = Box<dyn std::error::Error>;

// This macro generates test cases for a given list of SWFs.
macro_rules! swf_tests {
    ($($(#[$attr:meta])* ($name:ident, $path:expr, $num_frames:literal),)*) => {
        $(
        #[test]
        $(#[$attr])*
        fn $name() -> Result<(), Error> {
            test_swf(
                concat!("tests/swfs/", $path, "/test.swf"),
                $num_frames,
                concat!("tests/swfs/", $path, "/output.txt"),
                |_| Ok(()),
                |_| Ok(()),
            )
        }
        )*
    };
}

// This macro generates test cases for a given list of SWFs using `test_swf_approx`.
macro_rules! swf_tests_approx {
    ($($(#[$attr:meta])* ($name:ident, $path:expr, $num_frames:literal $(, $opt:ident = $val:expr)*),)*) => {
        $(
        #[test]
        $(#[$attr])*
        fn $name() -> Result<(), Error> {
            test_swf_approx(
                concat!("tests/swfs/", $path, "/test.swf"),
                $num_frames,
                concat!("tests/swfs/", $path, "/output.txt"),
                |actual, expected| assert_relative_eq!(actual, expected $(, $opt = $val)*),
                //$relative_epsilon,
                |_| Ok(()),
                |_| Ok(()),
            )
        }
        )*
    };
}

// List of SWFs to test.
// Format: (test_name, test_folder, number_of_frames_to_run)
// The test folder is a relative to core/tests/swfs
// Inside the folder is expected to be "test.swf" and "output.txt" with the correct output.
swf_tests! {
    (add_property, "avm1/add_property", 1),
    (as_transformed_flag, "avm1/as_transformed_flag", 3),
    (as_broadcaster, "avm1/as_broadcaster", 1),
    (as_broadcaster_initialize, "avm1/as_broadcaster_initialize", 1),
    (attach_movie, "avm1/attach_movie", 1),
    (function_base_clip, "avm1/function_base_clip", 2),
    (call, "avm1/call", 2),
    (color, "avm1/color", 1),
    (clip_events, "avm1/clip_events", 4),
    (unload_clip_event, "avm1/unload_clip_event", 2),
    (create_empty_movie_clip, "avm1/create_empty_movie_clip", 2),
    (empty_movieclip_can_attach_movies, "avm1/empty_movieclip_can_attach_movies", 1),
    (duplicate_movie_clip, "avm1/duplicate_movie_clip", 1),
    (mouse_listeners, "avm1/mouse_listeners", 1),
    (do_init_action, "avm1/do_init_action", 3),
    (execution_order1, "avm1/execution_order1", 3),
    (execution_order2, "avm1/execution_order2", 15),
    (execution_order3, "avm1/execution_order3", 5),
    (single_frame, "avm1/single_frame", 2),
    (looping, "avm1/looping", 6),
    (matrix, "avm1/matrix", 1),
    (point, "avm1/point", 1),
    (rectangle, "avm1/rectangle", 1),
    (date_is_special, "avm1/date_is_special", 1),
    (goto_advance1, "avm1/goto_advance1", 2),
    (goto_advance2, "avm1/goto_advance2", 2),
    (goto_both_ways1, "avm1/goto_both_ways1", 2),
    (goto_both_ways2, "avm1/goto_both_ways2", 3),
    (goto_frame, "avm1/goto_frame", 3),
    (goto_frame2, "avm1/goto_frame2", 5),
    (goto_frame_number, "avm1/goto_frame_number", 4),
    (goto_label, "avm1/goto_label", 4),
    (goto_methods, "avm1/goto_methods", 1),
    (goto_rewind1, "avm1/goto_rewind1", 4),
    (goto_rewind2, "avm1/goto_rewind2", 5),
    (goto_rewind3, "avm1/goto_rewind3", 2),
    (goto_execution_order, "avm1/goto_execution_order", 3),
    (goto_execution_order2, "avm1/goto_execution_order2", 2),
    (greaterthan_swf5, "avm1/greaterthan_swf5", 1),
    (greaterthan_swf8, "avm1/greaterthan_swf8", 1),
    (strictly_equals, "avm1/strictly_equals", 1),
    (tell_target, "avm1/tell_target", 3),
    (typeofs, "avm1/typeof", 1),
    (typeof_globals, "avm1/typeof_globals", 1),
    (closure_scope, "avm1/closure_scope", 1),
    (variable_args, "avm1/variable_args", 1),
    (custom_clip_methods, "avm1/custom_clip_methods", 3),
    (delete, "avm1/delete", 3),
    (selection, "avm1/selection", 1),
    (default_names, "avm1/default_names", 6),
    (array_trivial, "avm1/array_trivial", 1),
    (array_concat, "avm1/array_concat", 1),
    (array_slice, "avm1/array_slice", 1),
    (array_splice, "avm1/array_splice", 1),
    (array_properties, "avm1/array_properties", 1),
    (array_prototyping, "avm1/array_prototyping", 1),
    (array_vs_object_length, "avm1/array_vs_object_length", 1),
    (array_sort, "avm1/array_sort", 1),
    (array_enumerate, "avm1/array_enumerate", 1),
    (timeline_function_def, "avm1/timeline_function_def", 3),
    (root_global_parent, "avm1/root_global_parent", 3),
    (register_underflow, "avm1/register_underflow", 1),
    (object_prototypes, "avm1/object_prototypes", 1),
    (movieclip_prototype_extension, "avm1/movieclip_prototype_extension", 1),
    (movieclip_hittest, "avm1/movieclip_hittest", 1),
    (movieclip_hittest_shapeflag, "avm1/movieclip_hittest_shapeflag", 10),
    (movieclip_lockroot, "avm1/movieclip_lockroot", 10),
    #[ignore] (textfield_text, "avm1/textfield_text", 1),
    (recursive_prototypes, "avm1/recursive_prototypes", 2),
    (stage_object_children, "avm1/stage_object_children", 2),
    (has_own_property, "avm1/has_own_property", 1),
    (extends_chain, "avm1/extends_chain", 1),
    (is_prototype_of, "avm1/is_prototype_of", 1),
    #[ignore] (string_coercion, "avm1/string_coercion", 1),
    (lessthan_swf4, "avm1/lessthan_swf4", 1),
    (lessthan2_swf5, "avm1/lessthan2_swf5", 1),
    (lessthan2_swf6, "avm1/lessthan2_swf6", 1),
    (lessthan2_swf7, "avm1/lessthan2_swf7", 1),
    (logical_ops_swf4, "avm1/logical_ops_swf4", 1),
    (logical_ops_swf8, "avm1/logical_ops_swf8", 1),
    (movieclip_depth_methods, "avm1/movieclip_depth_methods", 3),
    (get_variable_in_scope, "avm1/get_variable_in_scope", 1),
    (movieclip_init_object, "avm1/movieclip_init_object", 1),
    (greater_swf6, "avm1/greater_swf6", 1),
    (greater_swf7, "avm1/greater_swf7", 1),
    (equals_swf4, "avm1/equals_swf4", 1),
    (equals2_swf5, "avm1/equals2_swf5", 1),
    (equals2_swf6, "avm1/equals2_swf6", 1),
    (equals2_swf7, "avm1/equals2_swf7", 1),
    (register_class, "avm1/register_class", 1),
    (register_and_init_order, "avm1/register_and_init_order", 1),
    (on_construct, "avm1/on_construct", 1),
    (set_variable_scope, "avm1/set_variable_scope", 1),
    (slash_syntax, "avm1/slash_syntax", 2),
    (strictequals_swf6, "avm1/strictequals_swf6", 1),
    (string_methods, "avm1/string_methods", 1),
    (string_ops_swf6, "avm1/string_ops_swf6", 1),
    (path_string, "avm1/path_string", 1),
    (global_is_bare, "avm1/global_is_bare", 1),
    (primitive_type_globals, "avm1/primitive_type_globals", 1),
    (primitive_instanceof, "avm1/primitive_instanceof", 1),
    (as2_oop, "avm1/as2_oop", 1),
    (xml, "avm1/xml", 1),
    (xml_namespaces, "avm1/xml_namespaces", 1),
    (xml_node_namespaceuri, "avm1/xml_node_namespaceuri", 1),
    (xml_node_weirdnamespace, "avm1/xml_node_weirdnamespace", 1),
    (xml_clone_expandos, "avm1/xml_clone_expandos", 1),
    (xml_has_child_nodes, "avm1/xml_has_child_nodes", 1),
    (xml_first_last_child, "avm1/xml_first_last_child", 1),
    (xml_parent_and_child, "avm1/xml_parent_and_child", 1),
    (xml_siblings, "avm1/xml_siblings", 1),
    (xml_attributes_read, "avm1/xml_attributes_read", 1),
    (xml_append_child, "avm1/xml_append_child", 1),
    (xml_append_child_with_parent, "avm1/xml_append_child_with_parent", 1),
    (xml_remove_node, "avm1/xml_remove_node", 1),
    (xml_insert_before, "avm1/xml_insert_before", 1),
    (xml_to_string, "avm1/xml_to_string", 1),
    (xml_to_string_comment, "avm1/xml_to_string_comment", 1),
    (xml_idmap, "avm1/xml_idmap", 1),
    (xml_ignore_comments, "avm1/xml_ignore_comments", 1),
    (xml_inspect_doctype, "avm1/xml_inspect_doctype", 1),
    #[ignore] (xml_inspect_xmldecl, "avm1/xml_inspect_xmldecl", 1),
    (xml_inspect_createmethods, "avm1/xml_inspect_createmethods", 1),
    (xml_inspect_parsexml, "avm1/xml_inspect_parsexml", 1),
    (funky_function_calls, "avm1/funky_function_calls", 1),
    (undefined_to_string_swf6, "avm1/undefined_to_string_swf6", 1),
    (define_function2_preload, "avm1/define_function2_preload", 1),
    (define_function2_preload_order, "avm1/define_function2_preload_order", 1),
    (mcl_as_broadcaster, "avm1/mcl_as_broadcaster", 1),
    (uncaught_exception, "avm1/uncaught_exception", 1),
    (uncaught_exception_bubbled, "avm1/uncaught_exception_bubbled", 1),
    (try_catch_finally, "avm1/try_catch_finally", 1),
    (try_finally_simple, "avm1/try_finally_simple", 1),
    (loadmovie, "avm1/loadmovie", 2),
    (loadmovienum, "avm1/loadmovienum", 2),
    (loadmovie_registerclass, "avm1/loadmovie_registerclass", 2),
    (loadmovie_method, "avm1/loadmovie_method", 2),
    (unloadmovie, "avm1/unloadmovie", 11),
    (unloadmovienum, "avm1/unloadmovienum", 11),
    (unloadmovie_method, "avm1/unloadmovie_method", 11),
    (mcl_loadclip, "avm1/mcl_loadclip", 11),
    (mcl_unloadclip, "avm1/mcl_unloadclip", 11),
    (mcl_getprogress, "avm1/mcl_getprogress", 6),
    (load_vars, "avm1/load_vars", 2),
    (loadvariables, "avm1/loadvariables", 3),
    (loadvariablesnum, "avm1/loadvariablesnum", 3),
    (loadvariables_method, "avm1/loadvariables_method", 3),
    (xml_load, "avm1/xml_load", 1),
    (with_return, "avm1/with_return", 1),
    (watch, "avm1/watch", 1),
    #[ignore] (watch_virtual_property, "avm1/watch_virtual_property", 1),
    (cross_movie_root, "avm1/cross_movie_root", 5),
    (roots_and_levels, "avm1/roots_and_levels", 1),
    (swf6_case_insensitive, "avm1/swf6_case_insensitive", 1),
    (swf7_case_sensitive, "avm1/swf7_case_sensitive", 1),
    (prototype_enumerate, "avm1/prototype_enumerate", 1),
    (stage_object_enumerate, "avm1/stage_object_enumerate", 1),
    (new_object_enumerate, "avm1/new_object_enumerate", 1),
    (as2_super_and_this_v6, "avm1/as2_super_and_this_v6", 1),
    (as2_super_and_this_v8, "avm1/as2_super_and_this_v8", 1),
    (as2_super_via_manual_prototype, "avm1/as2_super_via_manual_prototype", 1),
    (as1_constructor_v6, "avm1/as1_constructor_v6", 1),
    (as1_constructor_v7, "avm1/as1_constructor_v7", 1),
    (issue_710, "avm1/issue_710", 1),
    (issue_1086, "avm1/issue_1086", 1),
    (issue_1104, "avm1/issue_1104", 3),
    (function_as_function, "avm1/function_as_function", 1),
    (infinite_recursion_function, "avm1/infinite_recursion_function", 1),
    (infinite_recursion_function_in_setter, "avm1/infinite_recursion_function_in_setter", 1),
    (infinite_recursion_virtual_property, "avm1/infinite_recursion_virtual_property", 1),
    (edittext_font_size, "avm1/edittext_font_size", 1),
    (edittext_default_format, "avm1/edittext_default_format", 1),
    (edittext_leading, "avm1/edittext_leading", 1),
    #[ignore] (edittext_newlines, "avm1/edittext_newlines", 1),
    (edittext_html_entity, "avm1/edittext_html_entity", 1),
    #[ignore] (edittext_html_roundtrip, "avm1/edittext_html_roundtrip", 1),
    (edittext_newline_stripping, "avm1/edittext_newline_stripping", 1),
    (define_local, "avm1/define_local", 1),
    (textfield_properties, "avm1/textfield_properties", 1),
    (textfield_variable, "avm1/textfield_variable", 8),
    (error, "avm1/error", 1),
    (color_transform, "avm1/color_transform", 1),
    (with, "avm1/with", 1),
    (arguments, "avm1/arguments", 1),
    (prototype_properties, "avm1/prototype_properties", 1),
    (stage_object_properties_get_var, "avm1/stage_object_properties_get_var", 1),
    (set_interval, "avm1/set_interval", 20),
    (context_menu, "avm1/context_menu", 1),
    (context_menu_item, "avm1/context_menu_item", 1),
    (constructor_function, "avm1/constructor_function", 1),
    (global_array, "avm1/global_array", 1),
    (array_constructor, "avm1/array_constructor", 1),
    (array_apply, "avm1/array_constructor", 1),
    (object_function, "avm1/object_function", 1),
    (parse_int, "avm1/parse_int", 1),
    (bitmap_filter, "avm1/bitmap_filter", 1),
    (blur_filter, "avm1/blur_filter", 1),
    (date_constructor, "avm1/date/constructor", 1),
    (removed_clip_halts_script, "avm1/removed_clip_halts_script", 13),
    (date_utc, "avm1/date/UTC", 1),
    (date_set_date, "avm1/date/setDate", 1),
    (date_set_full_year, "avm1/date/setFullYear", 1),
    (date_set_hours, "avm1/date/setHours", 1),
    (date_set_milliseconds, "avm1/date/setMilliseconds", 1),
    (date_set_minutes, "avm1/date/setMinutes", 1),
    (date_set_month, "avm1/date/setMonth", 1),
    (date_set_seconds, "avm1/date/setSeconds", 1),
    (date_set_time, "avm1/date/setTime", 1),
    (date_set_utc_date, "avm1/date/setUTCDate", 1),
    (date_set_utc_full_year, "avm1/date/setUTCFullYear", 1),
    (date_set_utc_hours, "avm1/date/setUTCHours", 1),
    (date_set_utc_milliseconds, "avm1/date/setUTCMilliseconds", 1),
    (date_set_utc_minutes, "avm1/date/setUTCMinutes", 1),
    (date_set_utc_month, "avm1/date/setUTCMonth", 1),
    (date_set_utc_seconds, "avm1/date/setUTCSeconds", 1),
    (date_set_year, "avm1/date/setYear", 1),
    (this_scoping, "avm1/this_scoping", 1),
    (bevel_filter, "avm1/bevel_filter", 1),
    (as3_hello_world, "avm2/hello_world", 1),
    (as3_function_call, "avm2/function_call", 1),
    (as3_function_call_via_call, "avm2/function_call_via_call", 1),
    (as3_constructor_call, "avm2/constructor_call", 1),
    (as3_class_methods, "avm2/class_methods", 1),
    (as3_es3_inheritance, "avm2/es3_inheritance", 1),
    (as3_es4_inheritance, "avm2/es4_inheritance", 1),
    (as3_stored_properties, "avm2/stored_properties", 1),
    (as3_virtual_properties, "avm2/virtual_properties", 1),
    (as3_es4_oop_prototypes, "avm2/es4_oop_prototypes", 1),
    (as3_es4_method_binding, "avm2/es4_method_binding", 1),
    (as3_control_flow_bool, "avm2/control_flow_bool", 1),
    (as3_control_flow_stricteq, "avm2/control_flow_stricteq", 1),
    (as3_object_enumeration, "avm2/object_enumeration", 1),
    (as3_class_enumeration, "avm2/class_enumeration", 1),
    (as3_is_prototype_of, "avm2/is_prototype_of", 1),
    (as3_has_own_property, "avm2/has_own_property", 1),
    (as3_property_is_enumerable, "avm2/property_is_enumerable", 1),
    (as3_set_property_is_enumerable, "avm2/set_property_is_enumerable", 1),
    (as3_object_to_string, "avm2/object_to_string", 1),
    (as3_function_to_string, "avm2/function_to_string", 1),
    (as3_class_to_string, "avm2/class_to_string", 1),
    (as3_object_to_locale_string, "avm2/object_to_locale_string", 1),
    (as3_function_to_locale_string, "avm2/function_to_locale_string", 1),
    (as3_class_to_locale_string, "avm2/class_to_locale_string", 1),
    (as3_object_value_of, "avm2/object_value_of", 1),
    (as3_function_value_of, "avm2/function_value_of", 1),
    (as3_class_value_of, "avm2/class_value_of", 1),
    (as3_if_stricteq, "avm2/if_stricteq", 1),
    (as3_if_strictne, "avm2/if_strictne", 1),
    (as3_strict_equality, "avm2/strict_equality", 1),
    (as3_es4_interfaces, "avm2/es4_interfaces", 1),
    (as3_istype, "avm2/istype", 1),
    (as3_instanceof, "avm2/instanceof", 1),
    (as3_truthiness, "avm2/truthiness", 1),
    (as3_falsiness, "avm2/falsiness", 1),
    (as3_boolean_negation, "avm2/boolean_negation", 1),
    (as3_convert_boolean, "avm2/convert_boolean", 1),
    (as3_convert_number, "avm2/convert_number", 1),
    (as3_convert_integer, "avm2/convert_integer", 1),
    (as3_convert_uinteger, "avm2/convert_uinteger", 1),
    (as3_coerce_string, "avm2/coerce_string", 1),
    (as3_if_eq, "avm2/if_eq", 1),
    (as3_if_ne, "avm2/if_ne", 1),
    (as3_equals, "avm2/equals", 1),
    (as3_if_lt, "avm2/if_lt", 1),
    (as3_if_lte, "avm2/if_lte", 1),
    (as3_if_gte, "avm2/if_gte", 1),
    (as3_if_gt, "avm2/if_gt", 1),
    (as3_greaterequals, "avm2/greaterequals", 1),
    (as3_greaterthan, "avm2/greaterthan", 1),
    (as3_lessequals, "avm2/lessequals", 1),
    (as3_lessthan, "avm2/lessthan", 1),
    (nested_textfields_in_buttons, "avm1/nested_textfields_in_buttons", 1),
    (conflicting_instance_names, "avm1/conflicting_instance_names", 6),
    (button_children, "avm1/button_children", 1),
    (transform, "avm1/transform", 1),
    (target_path, "avm1/target_path", 1),
    (remove_movie_clip, "avm1/remove_movie_clip", 1),
    (as3_add, "avm2/add", 1),
    (as3_bitand, "avm2/bitand", 1),
    (as3_bitnot, "avm2/bitnot", 1),
    (as3_declocal, "avm2/declocal", 1),
    (as3_declocal_i, "avm2/declocal_i", 1),
    (as3_decrement, "avm2/decrement", 1),
    (as3_decrement_i, "avm2/decrement_i", 1),
    (as3_inclocal, "avm2/inclocal", 1),
    (as3_inclocal_i, "avm2/inclocal_i", 1),
    (as3_increment, "avm2/increment", 1),
    (as3_increment_i, "avm2/increment_i", 1),
    (as3_lshift, "avm2/lshift", 1),
    (as3_modulo, "avm2/modulo", 1),
    (as3_multiply, "avm2/multiply", 1),
    (as3_negate, "avm2/negate", 1),
    (as3_rshift, "avm2/rshift", 1),
    (as3_subtract, "avm2/subtract", 1),
    (as3_urshift, "avm2/urshift", 1),
    (as3_in, "avm2/in", 1),
    (as3_array_constr, "avm2/array_constr", 1),
    (as3_array_access, "avm2/array_access", 1),
    (as3_array_storage, "avm2/array_storage", 1),
    (as3_array_delete, "avm2/array_delete", 1),
    (as3_array_holes, "avm2/array_holes", 1),
    (as3_array_literal, "avm2/array_literal", 1),
    (as3_array_concat, "avm2/array_concat", 1),
    (as3_array_tostring, "avm2/array_tostring", 1),
    (as3_array_tolocalestring, "avm2/array_tolocalestring", 1),
    (as3_array_valueof, "avm2/array_valueof", 1),
    (as3_array_join, "avm2/array_join", 1),
    (as3_array_foreach, "avm2/array_foreach", 1),
    (as3_array_map, "avm2/array_map", 1),
    (as3_array_filter, "avm2/array_filter", 1),
    (as3_array_every, "avm2/array_every", 1),
    (as3_array_some, "avm2/array_some", 1),
    (as3_array_indexof, "avm2/array_indexof", 1),
    (as3_array_lastindexof, "avm2/array_lastindexof", 1),
    (as3_array_push, "avm2/array_push", 1),
    (as3_array_pop, "avm2/array_pop", 1),
    (as3_array_reverse, "avm2/array_reverse", 1),
    (as3_array_shift, "avm2/array_shift", 1),
    (as3_array_unshift, "avm2/array_unshift", 1),
    (as3_array_slice, "avm2/array_slice", 1),
    (as3_array_splice, "avm2/array_splice", 1),
    (as3_array_sort, "avm2/array_sort", 1),
    (as3_array_sorton, "avm2/array_sorton", 1),
    (as3_array_hasownproperty, "avm2/array_hasownproperty", 1),
    (stage_property_representation, "avm1/stage_property_representation", 1),
    (as3_timeline_scripts, "avm2/timeline_scripts", 3),
    (as3_movieclip_properties, "avm2/movieclip_properties", 4),
    (as3_movieclip_gotoandplay, "avm2/movieclip_gotoandplay", 5),
    (as3_movieclip_gotoandstop, "avm2/movieclip_gotoandstop", 5),
    (as3_movieclip_stop, "avm2/movieclip_stop", 5),
    (as3_movieclip_prev_frame, "avm2/movieclip_prev_frame", 5),
    (as3_movieclip_next_frame, "avm2/movieclip_next_frame", 5),
    (as3_movieclip_prev_scene, "avm2/movieclip_prev_scene", 5),
    (as3_movieclip_next_scene, "avm2/movieclip_next_scene", 5),
    (as3_framelabel_constr, "avm2/framelabel_constr", 5),
    (as3_movieclip_currentlabels, "avm2/movieclip_currentlabels", 5),
    (as3_scene_constr, "avm2/scene_constr", 5),
    (as3_movieclip_currentscene, "avm2/movieclip_currentscene", 5),
    (as3_movieclip_scenes, "avm2/movieclip_scenes", 5),
    (as3_movieclip_play, "avm2/movieclip_play", 5),
    (as3_movieclip_constr, "avm2/movieclip_constr", 1),
    (as3_lazyinit, "avm2/lazyinit", 1),
    (as3_trace, "avm2/trace", 1),
    (as3_displayobjectcontainer_getchildat, "avm2/displayobjectcontainer_getchildat", 1),
    (as3_displayobjectcontainer_getchildbyname, "avm2/displayobjectcontainer_getchildbyname", 1),
    (as3_displayobjectcontainer_addchild, "avm2/displayobjectcontainer_addchild", 1),
    (as3_displayobjectcontainer_addchildat, "avm2/displayobjectcontainer_addchildat", 1),
    (as3_displayobjectcontainer_removechild, "avm2/displayobjectcontainer_removechild", 1),
    (as3_displayobjectcontainer_removechild_timelinemanip_remove1, "avm2/displayobjectcontainer_removechild_timelinemanip_remove1", 7),
    (as3_displayobjectcontainer_addchild_timelinepull0, "avm2/displayobjectcontainer_addchild_timelinepull0", 7),
    (as3_displayobjectcontainer_addchild_timelinepull1, "avm2/displayobjectcontainer_addchild_timelinepull1", 7),
    (as3_displayobjectcontainer_addchild_timelinepull2, "avm2/displayobjectcontainer_addchild_timelinepull2", 7),
    (as3_displayobjectcontainer_addchildat_timelinelock0, "avm2/displayobjectcontainer_addchildat_timelinelock0", 7),
    (as3_displayobjectcontainer_addchildat_timelinelock1, "avm2/displayobjectcontainer_addchildat_timelinelock1", 7),
    (as3_displayobjectcontainer_addchildat_timelinelock2, "avm2/displayobjectcontainer_addchildat_timelinelock2", 7),
    (as3_displayobjectcontainer_contains, "avm2/displayobjectcontainer_contains", 5),
    (as3_displayobjectcontainer_getchildindex, "avm2/displayobjectcontainer_getchildindex", 5),
    (as3_displayobjectcontainer_removechildat, "avm2/displayobjectcontainer_removechildat", 1),
    (as3_displayobjectcontainer_removechildren, "avm2/displayobjectcontainer_removechildren", 5),
    (as3_displayobjectcontainer_setchildindex, "avm2/displayobjectcontainer_setchildindex", 1),
    (as3_displayobjectcontainer_swapchildren, "avm2/displayobjectcontainer_swapchildren", 1),
    (as3_displayobjectcontainer_swapchildrenat, "avm2/displayobjectcontainer_swapchildrenat", 1),
    (button_order, "avm1/button_order", 1),
    (as3_displayobjectcontainer_stopallmovieclips, "avm2/displayobjectcontainer_stopallmovieclips", 2),
    (as3_displayobjectcontainer_timelineinstance, "avm2/displayobjectcontainer_timelineinstance", 6),
    (as3_displayobject_alpha, "avm2/displayobject_alpha", 1),
    (as3_displayobject_x, "avm2/displayobject_x", 1),
    (as3_displayobject_y, "avm2/displayobject_y", 1),
    (as3_displayobject_name, "avm2/displayobject_name", 4),
    (as3_displayobject_parent, "avm2/displayobject_parent", 4),
    (as3_displayobject_root, "avm2/displayobject_root", 4),
    (as3_displayobject_visible, "avm2/displayobject_visible", 4),
    (as3_displayobject_hittestpoint, "avm2/displayobject_hittestpoint", 2),
    (as3_displayobject_hittestobject, "avm2/displayobject_hittestobject", 1),
}

// TODO: These tests have some inaccuracies currently, so we use approx_eq to test that numeric values are close enough.
// Eventually we can hopefully make some of these match exactly (see #193).
// Some will probably always need to be approx. (if they rely on trig functions, etc.)
swf_tests_approx! {
    (local_to_global, "avm1/local_to_global", 1, epsilon = 0.051),
    (stage_object_properties, "avm1/stage_object_properties", 6, epsilon = 0.051),
    (stage_object_properties_swf6, "avm1/stage_object_properties_swf6", 4, epsilon = 0.051),
    (movieclip_getbounds, "avm1/movieclip_getbounds", 1, epsilon = 0.051),
    (edittext_letter_spacing, "avm1/edittext_letter_spacing", 1, epsilon = 15.0), // TODO: Discrepancy in wrapping in letterSpacing = 0.1 test.
    (edittext_align, "avm1/edittext_align", 1, epsilon = 3.0),
    (edittext_margins, "avm1/edittext_margins", 1, epsilon = 5.0), // TODO: Discrepancy in wrapping.
    (edittext_tab_stops, "avm1/edittext_tab_stops", 1, epsilon = 5.0),
    (edittext_bullet, "avm1/edittext_bullet", 1, epsilon = 3.0),
    (edittext_underline, "avm1/edittext_underline", 1, epsilon = 4.0),
    (as3_coerce_string_precision, "avm2/coerce_string_precision", 1, max_relative = 30.0 * std::f64::EPSILON),
    (as3_divide, "avm2/divide", 1, epsilon = 0.0), // TODO: Discrepancy in float formatting.
    (as3_math, "avm2/math", 1, max_relative = 30.0 * std::f64::EPSILON),
    (as3_displayobject_height, "avm2/displayobject_height", 7, epsilon = 0.06), // TODO: height/width appears to be off by 1 twip sometimes
    (as3_displayobject_width, "avm2/displayobject_width", 7, epsilon = 0.06),
    (as3_displayobject_rotation, "avm2/displayobject_rotation", 1, epsilon = 0.0000000001),
}

#[test]
fn external_interface_avm1() -> Result<(), Error> {
    test_swf(
        "tests/swfs/avm1/external_interface/test.swf",
        1,
        "tests/swfs/avm1/external_interface/output.txt",
        |player| {
            player
                .lock()
                .unwrap()
                .add_external_interface(Box::new(ExternalInterfaceTestProvider::new()));
            Ok(())
        },
        |player| {
            let mut player_locked = player.lock().unwrap();

            let parroted =
                player_locked.call_internal_interface("parrot", vec!["Hello World!".into()]);
            player_locked.log_backend().avm_trace(&format!(
                "After calling `parrot` with a string: {:?}",
                parroted
            ));

            let mut nested = BTreeMap::new();
            nested.insert(
                "list".to_string(),
                vec![
                    "string".into(),
                    100.into(),
                    false.into(),
                    ExternalValue::Object(BTreeMap::new()),
                ]
                .into(),
            );

            let mut root = BTreeMap::new();
            root.insert("number".to_string(), (-500.1).into());
            root.insert("string".to_string(), "A string!".into());
            root.insert("true".to_string(), true.into());
            root.insert("false".to_string(), false.into());
            root.insert("null".to_string(), ExternalValue::Null);
            root.insert("nested".to_string(), nested.into());
            let result = player_locked
                .call_internal_interface("callWith", vec!["trace".into(), root.into()]);
            player_locked.log_backend().avm_trace(&format!(
                "After calling `callWith` with a complex payload: {:?}",
                result
            ));
            Ok(())
        },
    )
}

#[test]
fn timeout_avm1() -> Result<(), Error> {
    test_swf(
        "tests/swfs/avm1/timeout/test.swf",
        1,
        "tests/swfs/avm1/timeout/output.txt",
        |player| {
            player
                .lock()
                .unwrap()
                .set_max_execution_duration(Duration::from_secs(5));
            Ok(())
        },
        |_| Ok(()),
    )
}

/// Wrapper around string slice that makes debug output `{:?}` to print string same way as `{}`.
/// Used in different `assert*!` macros in combination with `pretty_assertions` crate to make
/// test failures to show nice diffs.
/// Courtesy of https://github.com/colin-kiegel/rust-pretty-assertions/issues/24
#[derive(PartialEq, Eq)]
#[doc(hidden)]
pub struct PrettyString<'a>(pub &'a str);

/// Make diff to display string as multi-line string
impl<'a> std::fmt::Debug for PrettyString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        pretty_assertions::assert_eq!(PrettyString($left.as_ref()), PrettyString($right.as_ref()));
    };
    ($left:expr, $right:expr, $message:expr) => {
        pretty_assertions::assert_eq!(
            PrettyString($left.as_ref()),
            PrettyString($right.as_ref()),
            $message
        );
    };
}

/// Loads an SWF and runs it through the Ruffle core for a number of frames.
/// Tests that the trace output matches the given expected output.
fn test_swf(
    swf_path: &str,
    num_frames: u32,
    expected_output_path: &str,
    before_start: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
    before_end: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut expected_output = std::fs::read_to_string(expected_output_path)?.replace("\r\n", "\n");

    // Strip a trailing newline if it has one.
    if expected_output.ends_with('\n') {
        expected_output = expected_output[0..expected_output.len() - "\n".len()].to_string();
    }

    let trace_log = run_swf(swf_path, num_frames, before_start, before_end)?;
    assert_eq!(
        trace_log, expected_output,
        "ruffle output != flash player output"
    );

    Ok(())
}

/// Loads an SWF and runs it through the Ruffle core for a number of frames.
/// Tests that the trace output matches the given expected output.
/// If a line has a floating point value, it will be compared approxinmately using the given epsilon.
fn test_swf_approx(
    swf_path: &str,
    num_frames: u32,
    expected_output_path: &str,
    approx_assert_fn: impl Fn(f64, f64),
    before_start: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
    before_end: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
) -> Result<(), Error> {
    let trace_log = run_swf(swf_path, num_frames, before_start, before_end)?;
    let mut expected_data = std::fs::read_to_string(expected_output_path)?;

    // Strip a trailing newline if it has one.
    if expected_data.ends_with('\n') {
        expected_data = expected_data[0..expected_data.len() - "\n".len()].to_string();
    }

    std::assert_eq!(
        trace_log.lines().count(),
        expected_data.lines().count(),
        "# of lines of output didn't match"
    );

    for (actual, expected) in trace_log.lines().zip(expected_data.lines()) {
        // If these are numbers, compare using approx_eq.
        if let (Ok(actual), Ok(expected)) = (actual.parse::<f64>(), expected.parse::<f64>()) {
            // NaNs should be able to pass in an approx test.
            if actual.is_nan() && expected.is_nan() {
                continue;
            }

            // TODO: Lower this epsilon as the accuracy of the properties improves.
            // if let Some(relative_epsilon) = relative_epsilon {
            //     assert_relative_eq!(
            //         actual,
            //         expected,
            //         epsilon = absolute_epsilon,
            //         max_relative = relative_epsilon
            //     );
            // } else {
            //     assert_abs_diff_eq!(actual, expected, epsilon = absolute_epsilon);
            // }
            approx_assert_fn(actual, expected);
        } else {
            assert_eq!(actual, expected);
        }
    }
    Ok(())
}

/// Loads an SWF and runs it through the Ruffle core for a number of frames.
/// Tests that the trace output matches the given expected output.
fn run_swf(
    swf_path: &str,
    num_frames: u32,
    before_start: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
    before_end: impl FnOnce(Arc<Mutex<Player>>) -> Result<(), Error>,
) -> Result<String, Error> {
    let base_path = Path::new(swf_path).parent().unwrap();
    let (mut executor, channel) = NullExecutor::new();
    let movie = SwfMovie::from_path(swf_path)?;
    let frame_time = 1000.0 / movie.header().frame_rate as f64;
    let trace_output = Rc::new(RefCell::new(Vec::new()));

    let player = Player::new(
        Box::new(NullRenderer),
        Box::new(NullAudioBackend::new()),
        Box::new(NullNavigatorBackend::with_base_path(base_path, channel)),
        Box::new(NullInputBackend::new()),
        Box::new(MemoryStorageBackend::default()),
        Box::new(NullLocaleBackend::new()),
        Box::new(TestLogBackend::new(trace_output.clone())),
    )?;
    player.lock().unwrap().set_root_movie(Arc::new(movie));
    player
        .lock()
        .unwrap()
        .set_max_execution_duration(Duration::from_secs(200));

    before_start(player.clone())?;

    for _ in 0..num_frames {
        player.lock().unwrap().run_frame();
        player.lock().unwrap().update_timers(frame_time);
        executor.poll_all().unwrap();
    }

    before_end(player)?;

    executor.block_all().unwrap();

    let trace = trace_output.borrow().join("\n");
    Ok(trace)
}

struct TestLogBackend {
    trace_output: Rc<RefCell<Vec<String>>>,
}

impl TestLogBackend {
    pub fn new(trace_output: Rc<RefCell<Vec<String>>>) -> Self {
        Self { trace_output }
    }
}

impl LogBackend for TestLogBackend {
    fn avm_trace(&self, message: &str) {
        self.trace_output.borrow_mut().push(message.to_string());
    }
}

#[derive(Default)]
pub struct ExternalInterfaceTestProvider {}

impl ExternalInterfaceTestProvider {
    pub fn new() -> Self {
        Default::default()
    }
}

fn do_trace(context: &mut UpdateContext<'_, '_, '_>, args: &[ExternalValue]) -> ExternalValue {
    context
        .log
        .avm_trace(&format!("[ExternalInterface] trace: {:?}", args));
    "Traced!".into()
}

fn do_ping(context: &mut UpdateContext<'_, '_, '_>, _args: &[ExternalValue]) -> ExternalValue {
    context.log.avm_trace("[ExternalInterface] ping");
    "Pong!".into()
}

fn do_reentry(context: &mut UpdateContext<'_, '_, '_>, _args: &[ExternalValue]) -> ExternalValue {
    context
        .log
        .avm_trace("[ExternalInterface] starting reentry");
    if let Some(callback) = context.external_interface.get_callback("callWith") {
        callback.call(
            context,
            "callWith",
            vec!["trace".into(), "successful reentry!".into()],
        )
    } else {
        ExternalValue::Null
    }
}

impl ExternalInterfaceProvider for ExternalInterfaceTestProvider {
    fn get_method(&self, name: &str) -> Option<Box<dyn ExternalInterfaceMethod>> {
        match name {
            "trace" => Some(Box::new(do_trace)),
            "ping" => Some(Box::new(do_ping)),
            "reentry" => Some(Box::new(do_reentry)),
            _ => None,
        }
    }

    fn on_callback_available(&self, _name: &str) {}
}
