/**
 * @file TODO
 * @author Polanas <ivanhilya@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check
const primitiveTypes = [
  'int',
  'float',
  'string',
  'any',
  'nil'
];

function return_type($) {
  return optional(seq('->', $._type));
}

module.exports = grammar({
  name: "lopa",
  extras: _ => [
    /\s/
  ],
  word: $ => $.identifier,

  rules: {
    // source_file: $ => $.one_line_string,
    source_file: $ => repeat($.item),
    item: $ => choice(
      $.item_fn,
      $.item_static,
      $.item_extern,
      $.item_inline,
      $.item_struct,
      $.item_enum,
      $.item_impl,
      $.item_use,
      $.item_mod,
    ),
    item_mod: $ => seq(
      'mod',
      $.identifier,
      choice(';', seq( '{', repeat($.item), '}'))
    ),
    item_use: $ => seq(
      'use',
      optional('::'),
      $.use_tree,
      ';'
    ),
    use_tree: $ => sepBy1('::', choice(
      $.identifier,
      seq('{', sepBy(',', $.use_tree), optional(','), '}'),
      '*',
    )),
    item_impl: $ => seq(
      'impl',
      $._type,
      $.impl_items,
    ),
    impl_items: $ => seq(
      '{',
      repeat($._impl_item),
      '}'
    ),
    _impl_item: $ => choice(
      $.impl_item_fn,
      $.item_static
    ),
    impl_item_fn: $ => seq(
      'fn',
      $.identifier,
      $.fn_param_list,
      return_type($),
      $.block,
    ),
    item_enum: $ => seq(
      'enum',
      $.identifier,
      $.fields
    ),
    item_struct: $ => seq(
      'struct',
      optional(seq('(', choice('value', 'gc', 'native'), ')')),
      $.identifier,
      $.fields,
    ),
    fields: $ => seq(
      '{',
      sepBy(',', $.field),
      optional(','),
      '}'
    ),
    field: $ => seq(
      $.identifier,
      ':',
      $._type,
      optional(seq('=', $._expression))
    ),
    item_inline: $ => seq(
      'inline',
      choice(
        seq('{', repeat($.inline_fn), '}'),
        $.inline_fn,
      )
    ),
    inline_fn: $ => seq(
      'fn',
      $.identifier,
      $.fn_param_list,
      return_type($),
      '=',
      $._string,
      ';'
    ),
    item_static: $ => seq(
      'static',
      $.identifier,
      optional(seq(':', $._type)),
      '=',
      $._expression,
      ';'
    ),
    item_extern: $ => seq(
      'extern',
      '(',
      choice('C', 'lua'),
      ')',
      choice(
        seq('{', repeat($.extern_fn), '}'),
        $.extern_fn,
      ),
    ),
    extern_fn: $ => seq(
      'fn',
      $.identifier,
      $.fn_param_list,
      return_type($),
      ';'
    ),
    item_fn: $ => seq(
      'fn',
      $.identifier,
      $.fn_param_list,
      return_type($),
      $.block,
    ),
    bare_fn_param_list: $ => seq(
      '(',
      sepBy(',', $.bare_fn_param),
      optional(','),
      ')'
    ),
    bare_fn_param: $ => seq(
      optional(seq($.identifier, ':')),
      $._type
    ),
    fn_param_list: $ => seq(
      '(',
      sepBy(',', $.fn_param),
      optional(','),
      ')'
    ),
    fn_param: $ => seq(
      $.identifier,
      ':',
      $._type,
      optional(seq('=', $._expression)),
    ),
    _type: $ => prec.left(1, seq(
      choice(
        alias(choice(...primitiveTypes), $.primitive_type),
        $.array_type,
        $.fn_type,
        $._path,
        $.tuple_type,
        $.union_type,
      ),
      optional('?'),
    )),
    tuple_type: $ => seq(
      '(',
      sepBy(',', $._type),
      optional(','),
      ')',
    ),
    union_type: $ => prec.left(1, seq(
      $._type,
      '|',
      $._type,
    )),
    _path: $ => choice(
      'self',
      $.identifier,
      $.scoped_identifier
    ),
    scoped_identifier: $ => seq(
      optional(choice(
        $._path
      )),
      '::',
      $.identifier,
    ),
    array_type: $ => seq(
      '[',
      $._type,
      ']'
    ),
    fn_type: $ => seq(
      'fn',
      $.bare_fn_param_list,
      return_type($),
    ),
    fn_kind: $ => choice('fn', 'co'),
    _statement: $ => choice(
      $.empty_statement,
    ),
    empty_statement: _ => ';',
    _expression: $ => choice(
      $.identifier,
      $.number,
      $._string,
    ),
    block: $ => seq(
      '{',
      repeat($._statement),
      '}'
    ),
    _string: $ => choice(
      $.one_line_string,
      $.multiline_string
    ),
    one_line_string: _ => seq(
      '"',
      token.immediate(/(?:[^"\\]|\\.)*/),
      '"',
    ),
    multiline_string: _ => seq(
      '"""',
      token.immediate(/(?:[^"\\]|\\.)*/),
      '"""',
    ),
    escape_sequence: _ => token.immediate(/\\(.|\n)/),
    number: _ => /[\d][\d|_]*(.[\d]+)?/,
    identifier: _ => /[_]?[A-Za-z_][0-9A-Za-z_]*/,
  }
});

/**
 * Creates a rule to match one or more of the rules separated by the separator.
 *
 * @param {RuleOrLiteral} sep - The separator to use.
 * @param {RuleOrLiteral} rule
 *
 * @returns {SeqRule}
 */
function sepBy1(sep, rule) {
  return seq(rule, repeat(seq(sep, rule)));
}


/**
 * Creates a rule to optionally match one or more of the rules separated by the separator.
 *
 * @param {RuleOrLiteral} sep - The separator to use.
 * @param {RuleOrLiteral} rule
 *
 * @returns {ChoiceRule}
 */
function sepBy(sep, rule) {
  return optional(sepBy1(sep, rule));
}
