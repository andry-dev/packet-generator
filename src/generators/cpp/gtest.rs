use stringcase::Caser;

use crate::{
    generators::{Addon, AddonOutput, GenerationError, cpp::GlazeAddon},
    intermediate::{Definition, DefinitionRegistry, Json},
};

#[derive(Debug)]
pub struct GoogleTestAddon {}

impl Addon for GoogleTestAddon {
    type For = GlazeAddon;

    fn postamble(
        &self,
        registry: &crate::intermediate::DefinitionRegistry,
    ) -> Option<Result<AddonOutput<'_>, GenerationError>> {
        let jsons: Result<Vec<String>, GenerationError> = registry
            .all_definitions()
            .filter_map(|def| match registry.get(def) {
                Definition::Json(json) => Some(generate_tests(registry, json)),
                _ => None,
            })
            .collect();

        match jsons {
            Ok(_json) => {
                // #include <gtest/gtest.h>
                // #include <mst_gatcha.hpp>
                //
                // using namespace date;
                // using namespace std;
                todo!();
            }

            Err(e) => Some(Err(e)),
        }
    }
}

fn generate_tests(_registry: &DefinitionRegistry, json: &Json) -> Result<String, GenerationError> {
    let test_name = json.name.to_snake_case();
    let _class_name = json.name.to_pascal_case();

    Ok(format!(
        r#"
TEST(packetgen, can_deserialize_json_{test_name})
	constexpr auto p = 2015_y/March/22;

	GachaMstData d1;
	d1.bg_img = "ayo test";
	d1.start_date = tp{{ sys_days{{p}} + chrono::hours{{3}} + chrono::minutes{{42}} + chrono::seconds{{14}} }};
	d1.once_day_flag = true;

	GachaMst d;
	d.data.emplace_back(d1);

	d1.bg_img = "blabla test2";
	d1.once_day_flag = false;
	d.data.emplace_back(d1);

	std::string buffer;
	const auto& ec = glz::write_json(d, buffer);

	ASSERT_EQ(ec.ec, glz::error_code::none);
	ASSERT_EQ(buffer, "{{\"5Y4GJeo3\":[{{\"1Dg0vUX3\":\"ayo test\",\"W9ABuJj2\":\"\",\"3sdHQb69\":\"\",\"gVSj32QH\":\"\",\"qp37xTDh\":\"\",\"W2c9g0Je\":\"\",\"uKYf13AH\":\"\",\"SzV0Nps7\":\"1970-01-01 00:00:00\",\"v9TR3cDz\":\"\",\"TCnm1F4v\":0,\"7Ffmi96v\":0,\"4N27mkt1\":\"\",\"J3stQ7jd\":0,\"03UGMHxF\":0,\"4tswNoV9\":\"1\",\"yu18xScw\":0,\"qA7M9EjP\":\"2015-03-22 03:42:14\",\"2HY3jpgu\":\"\",\"S1oz60Hc\":0}},{{\"1Dg0vUX3\":\"blabla test2\",\"W9ABuJj2\":\"\",\"3sdHQb69\":\"\",\"gVSj32QH\":\"\",\"qp37xTDh\":\"\",\"W2c9g0Je\":\"\",\"uKYf13AH\":\"\",\"SzV0Nps7\":\"1970-01-01 00:00:00\",\"v9TR3cDz\":\"\",\"TCnm1F4v\":0,\"7Ffmi96v\":0,\"4N27mkt1\":\"\",\"J3stQ7jd\":0,\"03UGMHxF\":0,\"4tswNoV9\":\"0\",\"yu18xScw\":0,\"qA7M9EjP\":\"2015-03-22 03:42:14\",\"2HY3jpgu\":\"\",\"S1oz60Hc\":0}}]}}");

    "#
    ))
}
