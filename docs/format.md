This file outlines my current thoughts on the design of TED's files format (not a typo).

# Design principals:
- use standard file formats, play nice with standard tools
- expose the model to the user
- don't break the (simplicity and interoperability of) csv

# A usecase walkthrough.
## 1. Your webscraper produces a file prices.csv
You open the file in ted, change a value, and save. prices.csv is updated, no other changes.

## 2. You change the formatting of a column to show percentages.
You save the file, prices.csv is unchanged, but a new file prices.json has been created. It looks like this:
```
{
    "meta" : [
        {
            "row" : 1,
            "header" : true
        }
    ]
    "format": [
        {
            "column" : 3,
            "display" : "percent"
        }
    ]
}
```

> NOTE: meta header will likely end up in the .toml (hopefully autodetected), instead of / in addition to here.

## 3. You add a formula column summing column A and B
You save the file and a new line is added to prices.json
```
{
    ...
    "columns" : [
        {
            "show" : 5
            "formula" : "A# + B#"
        }
    ]
}
```

## 4. You add another sheet to the spreadsheet, called products
You add a column of product id's and a column of their name. You save.
2 new files are created. products.csv, and prices.toml.
products.csv has your data and prices.toml look like this: 
```
[prices]
index = 0
path = "prices.csv"

[products]
index = 1
path = "products.csv"
```

## 5. You have another file on disk, which you want to load as a dataset.
You create a symlink to the data. And add to prices.toml
```
[my_imported_data]
index = 3
readonly = "true"
path = "my_imported_data.csv"
```
It shows in the ted like a regular sheet, but displays `linked` and `ro`.
Instead of doing it manually you could have ran `:ln ../../mylinkeddata.csv` in ted.

### Note: 
It might be the case that this is a bad system for supporting windows users, so perhaps prices.toml would support a `link = "../../my_data.csv"` directive. 
alternatively perhaps ted would just be able to parse symlinks on windows and load the referenced file...
why not both, with a command `:convert_links <native|ted>` 

## 6. You have a script which loads some data from the internet and scrapes it to produce a table, you want this data as part of the spreadsheet.
Add a line to prices.toml.
```
[scraperdata]
index = 4
path = "scraper.csv"
automated = true

[program.scraper]
exec = "./scrape.sh ${updates.path}"
updates = "scraperdata"
```

There would also be a format for things where you don't even want the data saved. Instead loaded every time ted is invoked. (Rarely desireable, but supported)
```
[scraperdata]
index = 4
exec = "./scrape.sh" #assuming scrape.sh outputs csv to std out.
```

> Note: Ted's support for script generated datasets can be used to query databases!. However, in the future we hope to have builtin support for database QUERIES and edits!

## 7. You have a csv that is tab seperated.
When importing the document, it should be autodetected, and will be recorded in the .toml if/when one is created.
You of course, can do that manually.
```
[tab_seperated_sheet]
index = 5 
path = "tsv_sheet.csv"

[tab_seperated_sheet.format]
column_seperator = "\t"
```

NOTE: unsolved problem, how to deal with formatting of the toml? in the example above, there are many ways to set tab_seperated_sheet.format.column_seperator.
If you set it manually, will ted overwrite it with default format the next time it needs to update the toml. 
TODO: implement pandorica transform for toml ;)

## 8. You want a pivot table.
Ted is built around the concept of: 1 table, 1 sheet, 1 file (well, techincally 2, one csv for data, one json for metadata).
Pivot tables are no different, except that they have no data file. 
Creating a pivot table named "mypivot" based on the "prices" csv will create a file mypivot.json
```
{
    "pivot" : {
        "rows" : []
        "columns" : []
        "data" : []
        "filter" : []
    }
}
```

because pivot tables refer to another sheet, the toml file must be updated as well, specifying the datasource.
we do not specify the data range, because in ted "1 table = 1 sheet", the range will always be the full table. 
(note you can always create a derivative table with a subset rows/columns, and use that as the pivot source, if you are in the very abnormal case where you don't want all rows + all columns available to the pivot)
We don't worry about heading here either, since the heading is metadata associated with the source table.

```
[mypivot]
pivot = "prices"
```

> NOTE: You may have noticed by now, that the general devision of labor between json and toml files is to put relationships and identifying information in the toml, and formatting,transformations,config,etc. in the json. This topic would need a lot of consideration and effort to pin down as the spec is developed.
> The main point of certainty is: 1 toml = 1 document (compare to 1 excel file) and 1 json = 1 sheet. 
> Having multible toml's in a folder is probably allowed, and perhaps its even permissible for them to share some csv/json files!

Adding calculated fields to the pivot tables output is the same as it is for a regular table (see 3), it gets put in the json in the same place.

## 9. And now, you want to send your amazing spreadsheet to someone.
Conveniently, your data is still just in csv's so if thats all they need, send them that. 
But if they are a fellow TEd enjoyer, you can send them your spreadsheet the typical unix way: as a tar archive.

And TEd should be able to open and edit this achive file, and save it back to the tar file.
And you can extract the file if you ever need access to the internals, and TEd doesn't care.

Further, TEd would hopefully support the common compression algorithms both on the tar archive, and on the csv's themselves.
Ideally, you should be able to compress prices.csv into prices.csv.lz4, and then open the project in TEd without even change the toml.